use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use rusqlite::{Connection, Result as SqliteResult};
use crate::error_log::log_error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub id: i64,
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<String>,
    pub timestamp: String,
}

fn get_db_path(cwd: &PathBuf, project_name: &str) -> PathBuf {
    cwd.join("projects").join(project_name).join("builder.db")
}

fn get_attachs_dir(cwd: &PathBuf, project_name: &str) -> PathBuf {
    cwd.join("projects").join(project_name).join("attachs")
}

fn init_database(db_path: &PathBuf) -> SqliteResult<Connection> {
    let conn = Connection::open(db_path)?;
    
    conn.execute(
        "CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            attachments TEXT,
            timestamp TEXT NOT NULL
        )",
        [],
    )?;
    
    Ok(conn)
}

fn load_messages_from_db(db_path: &PathBuf) -> Result<Vec<Message>, String> {
    if !db_path.exists() {
        return Ok(Vec::new());
    }
    
    let conn = init_database(db_path)
        .map_err(|e| format!("数据库连接失败: {}", e))?;
    
    let mut stmt = conn
        .prepare("SELECT id, role, content, attachments, timestamp FROM messages ORDER BY id ASC")
        .map_err(|e| format!("查询失败: {}", e))?;
    
    let messages = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let role: String = row.get(1)?;
            let content: String = row.get(2)?;
            let attachments_str: Option<String> = row.get(3)?;
            let timestamp: String = row.get(4)?;
            
            let attachments: Vec<String> = attachments_str
                .map(|s| serde_json::from_str(&s).unwrap_or_default())
                .unwrap_or_default();
            
            Ok(Message {
                id,
                role,
                content,
                attachments,
                timestamp,
            })
        })
        .map_err(|e| format!("读取消息失败: {}", e))?
        .collect::<SqliteResult<Vec<Message>>>()
        .map_err(|e| format!("收集消息失败: {}", e))?;
    
    Ok(messages)
}

/// 加载对话记录
#[tauri::command]
pub async fn load_conversation(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
) -> Result<Vec<Message>, String> {
    let db_path = get_db_path(&cwd, &project_name);
        
    if !db_path.exists() {
        return Ok(Vec::new());
    }
    
    let conn = init_database(&db_path).map_err(|e| {
        let err_msg = format!("数据库连接失败: {}", e);
        log_error(&cwd, &err_msg);
        err_msg
    })?;
    
    let mut stmt = conn.prepare("SELECT id, role, content, attachments, timestamp FROM messages ORDER BY id ASC").map_err(|e| {
        let err_msg = format!("查询失败: {}", e);
        log_error(&cwd, &err_msg);
        err_msg
    })?;
    
    let messages = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let role: String = row.get(1)?;
            let content: String = row.get(2)?;
            let attachments_str: Option<String> = row.get(3)?;
            let timestamp: String = row.get(4)?;
            
            let attachments: Vec<String> = attachments_str
                .map(|s| serde_json::from_str(&s).unwrap_or_default())
                .unwrap_or_default();
            
            Ok(Message {
                id,
                role,
                content,
                attachments,
                timestamp,
            })
        })
        .map_err(|e| {
            let err_msg = format!("读取消息失败: {}", e);
            log_error(&cwd, &err_msg);
            err_msg
        })?
        .collect::<SqliteResult<Vec<Message>>>()
        .map_err(|e| {
            let err_msg = format!("收集消息失败: {}", e);
            log_error(&cwd, &err_msg);
            err_msg
        })?;
    
    Ok(messages)
}

/// 保存对话记录
#[tauri::command]
pub async fn save_conversation(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    messages: Vec<Message>,
) -> Result<(), String> {
    let db_path = get_db_path(&cwd, &project_name);
    
    // 确保目录存在
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            let err_msg = format!("创建目录失败: {}", e);
            log_error(&cwd, &err_msg);
            err_msg
        })?;
    }
    
    let conn = init_database(&db_path).map_err(|e| {
        let err_msg = format!("数据库连接失败: {}", e);
        log_error(&cwd, &err_msg);
        err_msg
    })?;
    
    // 清空现有消息
    conn.execute("DELETE FROM messages", []).map_err(|e| {
        let err_msg = format!("清空消息失败: {}", e);
        log_error(&cwd, &err_msg);
        err_msg
    })?;
    
    // 插入新消息
    let mut stmt = conn.prepare("INSERT INTO messages (role, content, attachments, timestamp) VALUES (?1, ?2, ?3, ?4)").map_err(|e| {
        let err_msg = format!("准备语句失败: {}", e);
        log_error(&cwd, &err_msg);
        err_msg
    })?;
    
    for msg in messages {
        let attachments_str = if msg.attachments.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&msg.attachments).unwrap_or_default())
        };
        
        // 确保role是有效值
        let role = msg.role.to_lowercase();
        
        stmt.execute([
            &role,
            &msg.content,
            &attachments_str.unwrap_or_default(),
            &msg.timestamp,
        ]).map_err(|e| {
            let err_msg = format!("插入消息失败: {}", e);
            log_error(&cwd, &err_msg);
            err_msg
        })?;
    }
    
    Ok(())
}

/// 保存附件
#[tauri::command]
pub async fn save_attachment(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    file_path: String,
) -> Result<String, String> {
    let attachs_dir = get_attachs_dir(&cwd, &project_name);
    
    // 创建附件目录
    tokio::fs::create_dir_all(&attachs_dir)
        .await
        .map_err(|e| format!("创建附件目录失败: {}", e))?;
    
    // 获取文件名
    let file_name = file_path
        .rsplit(|c| c == '/' || c == '\\')
        .next()
        .unwrap_or("file");
    let dest_path = attachs_dir.join(file_name);
    
    // 复制文件
    tokio::fs::copy(&file_path, &dest_path)
        .await
        .map_err(|e| format!("复制文件失败: {}", e))?;
    
    Ok(dest_path.to_string_lossy().to_string())
}

/// 与AI对话
#[tauri::command]
pub async fn chat_with_ai(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    message: String,
    attachments: Vec<String>,
    system_prompt: String,
) -> Result<String, String> {
    use crate::config::AppConfig;
    
    // 加载配置
    let config_path = cwd.join("config.yaml");
    let config_content = tokio::fs::read_to_string(&config_path)
        .await
        .map_err(|e| format!("读取配置失败: {}", e))?;
    
    let config: AppConfig = serde_yaml::from_str(&config_content)
        .map_err(|e| format!("解析配置失败: {}", e))?;
    
    // 加载历史对话
    let db_path = get_db_path(&cwd, &project_name);
    let history = load_messages_from_db(&db_path)?;
    
    // 创建HTTP客户端（在处理附件之前，因为上传文件需要用到）
    let client = reqwest::Client::new();
    
    // 构建消息列表
    let mut messages_vec: Vec<serde_json::Value> = vec![
        serde_json::json!({
            "role": "system",
            "content": system_prompt
        })
    ];
    
    // 添加历史消息
    for msg in history {
        messages_vec.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content
        }));
    }
    
    // 处理当前消息和附件
    if attachments.is_empty() {
        messages_vec.push(serde_json::json!({
            "role": "user",
            "content": message
        }));
    } else {
        // 处理附件
        let mut content_items: Vec<serde_json::Value> = vec![
            serde_json::json!({
                "type": "text",
                "text": message
            })
        ];
        
        for attachment_path in attachments {
            let path = std::path::PathBuf::from(&attachment_path);
            if !path.exists() {
                continue;
            }
            
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            
            // 只支持文本文件（txt, md）
            let text_types = ["txt", "md"];
            if text_types.contains(&extension.as_str()) {
                let file_content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| format!("读取附件失败: {}", e))?;
                content_items.push(serde_json::json!({
                    "type": "text",
                    "text": format!("\n\n---\n附件: {}\n---\n{}", file_name, file_content)
                }));
            }
            // 其他文件类型忽略
        }
        
        messages_vec.push(serde_json::json!({
            "role": "user",
            "content": content_items
        }));
    }
    
    // 调用LLM API
    let mut request = client
        .post(format!("{}/chat/completions", config.llm.endpoint))
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .header("Content-Type", "application/json");
    
    for header in &config.llm.extra_headers {
        request = request.header(&header.key, &header.value);
    }
    
    let response = request
        .json(&serde_json::json!({
            "model": config.llm.model,
            "messages": messages_vec,
            "temperature": 0.7
        }))
        .send()
        .await
        .map_err(|e| format!("API请求失败: {}", e))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API错误: {}", error_text));
    }
    
    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;
    
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("无法提取响应内容")?
        .to_string();
    
    Ok(content)
}

/// 生成需求文档
#[tauri::command]
pub async fn generate_requirements(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
) -> Result<String, String> {
    use crate::config::AppConfig;
    
    // 加载配置
    let config_path = cwd.join("config.yaml");
    let config_content = tokio::fs::read_to_string(&config_path)
        .await
        .map_err(|e| format!("读取配置失败: {}", e))?;
    
    let config: AppConfig = serde_yaml::from_str(&config_content)
        .map_err(|e| format!("解析配置失败: {}", e))?;
    
    // 加载对话记录
    let db_path = get_db_path(&cwd, &project_name);
    let history = load_messages_from_db(&db_path)?;
    
    if history.is_empty() {
        return Err("没有对话记录".to_string());
    }
    
    // 构建对话摘要
    let conversation_summary: String = history
        .iter()
        .map(|msg| format!("【{}】{}", msg.role, msg.content))
        .collect::<Vec<_>>()
        .join("\n\n");
    
    // 加载附件内容
    let attachs_dir = get_attachs_dir(&cwd, &project_name);
    let mut attachments_content = String::new();
    
    // 只支持文本文件
    let allowed_extensions = ["txt", "md"];
    
    if attachs_dir.exists() {
        let mut entries = tokio::fs::read_dir(&attachs_dir)
            .await
            .map_err(|e| format!("读取附件目录失败: {}", e))?;
        
        while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
            let path = entry.path();
            if path.is_file() {
                let extension = path.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                
                // 只处理文本文件
                if !allowed_extensions.contains(&extension.as_str()) {
                    continue;
                }
                
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    attachments_content.push_str(&format!("\n---\n附件: {}\n---\n{}\n", name, content));
                }
            }
        }
    }
    
    // 构建完整提示词
    let prompt = if attachments_content.is_empty() {
        format!(r#"根据以下对话内容，生成一份完整的演示需求文档。

对话记录：
{}

请按照以下格式生成需求文档：

【目标】
明确演示的核心目标，包括：
- 主要目的是什么（说服/教育/汇报/激励）
- 希望听众在演示结束后做什么或想什么
- 成功的标准是什么

【受众】
描述目标听众，包括：
- 听众背景、知识水平
- 听众的关注点和痛点
- 可能的疑虑或反对意见

【核心信息】
提炼关键信息，包括：
- 核心观点（一句话概括）
- 关键论据（3-5个要点）
- 支撑数据或案例

【场景约束】
列出演示场景和约束，包括：
- 演示场合和时间限制
- 品牌或视觉要求
- 其他特殊要求

注意：请直接输出需求文档内容，不要添加额外的说明或解释。"#, conversation_summary)
    } else {
        format!(r#"根据以下对话内容和附件信息，生成一份完整的演示需求文档。

对话记录：
{}

附件内容：
{}

请按照以下格式生成需求文档：

【目标】
明确演示的核心目标，包括：
- 主要目的是什么（说服/教育/汇报/激励）
- 希望听众在演示结束后做什么或想什么
- 成功的标准是什么

【受众】
描述目标听众，包括：
- 听众背景、知识水平
- 听众的关注点和痛点
- 可能的疑虑或反对意见

【核心信息】
提炼关键信息，包括：
- 核心观点（一句话概括）
- 关键论据（3-5个要点）
- 支撑数据或案例

【场景约束】
列出演示场景和约束，包括：
- 演示场合和时间限制
- 品牌或视觉要求
- 其他特殊要求

注意：请直接输出需求文档内容，不要添加额外的说明或解释。"#, conversation_summary, attachments_content)
    };
    
    // 调用LLM生成需求
    let client = reqwest::Client::new();
    
    let mut request = client
        .post(format!("{}/chat/completions", config.llm.endpoint))
        .header("Authorization", format!("Bearer {}", config.llm.api_key))
        .header("Content-Type", "application/json");
    
    for header in &config.llm.extra_headers {
        request = request.header(&header.key, &header.value);
    }
    
    let response = request
        .json(&serde_json::json!({
            "model": config.llm.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3
        }))
        .send()
        .await
        .map_err(|e| format!("API请求失败: {}", e))?;
    
    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API错误: {}", error_text));
    }
    
    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;
    
    let requirements = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("无法提取响应内容")?
        .to_string();
    
    Ok(requirements)
}

/// 保存需求文档
#[tauri::command]
pub async fn save_requirements(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    requirements: String,
) -> Result<(), String> {
    let requirements_path = cwd
        .join("projects")
        .join(&project_name)
        .join("requirements.md");
    
    tokio::fs::write(&requirements_path, requirements)
        .await
        .map_err(|e| format!("保存需求文档失败: {}", e))?;
    
    Ok(())
}

/// 加载需求文档
#[tauri::command]
pub async fn load_requirements(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
) -> Result<String, String> {
    let requirements_path = cwd
        .join("projects")
        .join(&project_name)
        .join("requirements.md");
    
    if !requirements_path.exists() {
        return Ok(String::new());
    }
    
    let content = tokio::fs::read_to_string(&requirements_path)
        .await
        .map_err(|e| format!("读取需求文档失败: {}", e))?;
    
    Ok(content)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub name: String,
    pub path: String,
    #[serde(rename = "type")]
    pub file_type: String,
}

/// 保存附件列表
#[tauri::command]
pub async fn save_attachments_list(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    attachments: Vec<AttachmentInfo>,
) -> Result<(), String> {
    let attachments_path = cwd
        .join("projects")
        .join(&project_name)
        .join("attachments.json");
    
    let content = serde_json::to_string_pretty(&attachments)
        .map_err(|e| format!("序列化附件列表失败: {}", e))?;
    
    tokio::fs::write(&attachments_path, content)
        .await
        .map_err(|e| format!("保存附件列表失败: {}", e))?;
    
    Ok(())
}

/// 加载附件列表
#[tauri::command]
pub async fn load_attachments_list(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
) -> Result<Vec<AttachmentInfo>, String> {
    let attachments_path = cwd
        .join("projects")
        .join(&project_name)
        .join("attachments.json");
    
    if !attachments_path.exists() {
        return Ok(Vec::new());
    }
    
    let content = tokio::fs::read_to_string(&attachments_path)
        .await
        .map_err(|e| format!("读取附件列表失败: {}", e))?;
    
    let attachments: Vec<AttachmentInfo> = serde_json::from_str(&content)
        .map_err(|e| format!("解析附件列表失败: {}", e))?;
    
    Ok(attachments)
}

/// 列出项目 attachs 目录中的所有附件
#[tauri::command]
pub async fn list_project_attachments(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
) -> Result<Vec<AttachmentInfo>, String> {
    let attachs_dir = get_attachs_dir(&cwd, &project_name);
    
    if !attachs_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut attachments = Vec::new();
    
    let mut entries = tokio::fs::read_dir(&attachs_dir)
        .await
        .map_err(|e| format!("读取附件目录失败: {}", e))?;
    
    // 只支持文本文件
    let allowed_extensions = ["txt", "md"];
    
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        let path = entry.path();
        if path.is_file() {
            let extension = path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            
            // 只处理允许的文件类型
            if !allowed_extensions.contains(&extension.as_str()) {
                continue;
            }
            
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            attachments.push(AttachmentInfo {
                name,
                path: path.to_string_lossy().to_string(),
                file_type: "document".to_string(),
            });
        }
    }
    
    // 按文件名排序
    attachments.sort_by(|a, b| a.name.cmp(&b.name));
    
    Ok(attachments)
}

#[allow(dead_code)]
fn get_file_type(extension: &str) -> String {
    // 只支持文本文件
    let text_types = ["txt", "md"];
    
    if text_types.contains(&extension) {
        "document".to_string()
    } else {
        "other".to_string()
    }
}

/// 删除附件
#[tauri::command]
pub async fn delete_attachment(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    attachment_name: String,
) -> Result<(), String> {
    let attachs_dir = get_attachs_dir(&cwd, &project_name);
    let file_path = attachs_dir.join(&attachment_name);
    
    if !file_path.exists() {
        return Err(format!("附件不存在: {}", attachment_name));
    }
    
    tokio::fs::remove_file(&file_path)
        .await
        .map_err(|e| format!("删除附件失败: {}", e))?;
    
    Ok(())
}

/// 读取附件内容（用于生成需求时参考）
/// TODO: 未来可用于前端单独读取附件内容
#[allow(dead_code)]
#[tauri::command]
pub async fn read_attachment_content(
    cwd: State<'_, Arc<PathBuf>>,
    project_name: String,
    attachment_name: String,
) -> Result<String, String> {
    let attachs_dir = get_attachs_dir(&cwd, &project_name);
    let file_path = attachs_dir.join(&attachment_name);
    
    if !file_path.exists() {
        return Err(format!("附件不存在: {}", attachment_name));
    }
    
    let extension = file_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    // 对于文本类文件，直接读取内容
    let text_types = ["txt", "md"];
    if text_types.contains(&extension.as_str()) {
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| format!("读取附件失败: {}", e))?;
        return Ok(content);
    }
    
    // 对于PDF和Word文档，返回提示信息
    // TODO: 可以集成PDF解析库来提取文本
    Ok(format!("[附件: {}]", attachment_name))
}
