export default {
  common: {
    save: '保存',
    cancel: '取消',
    delete: '删除',
    edit: '编辑',
    create: '创建',
    confirm: '确定',
    loading: '加载中...',
    success: '成功',
    error: '错误',
    warning: '警告',
    search: '搜索',
    reset: '重置',
    upload: '上传',
    download: '下载',
    open: '打开',
    close: '关闭',
    back: '返回',
    next: '下一步',
    previous: '上一步',
    generate: '生成',
    regenerate: '重新生成',
    clear: '清空',
    copy: '复制',
    paste: '粘贴',
    refresh: '刷新'
  },
  menu: {
    projects: '项目',
    brainstorm: '思路',
    outline: '大纲',
    pages: 'PPTs',
    skeletons: '结构',
    templates: '模板',
    styles: '风格',
    api: 'API',
    logs: '日志',
    about: '关于'
  },
  project: {
    title: '项目列表',
    create: '新建项目',
    topic: '项目主题',
    topicPlaceholder: '输入主题，如：2026年度销售报告',
    open: '打开',
    folder: '文件夹',
    deleteConfirm: '确定要删除此项目吗？',
    deleteDesc: '将删除项目文件夹及所有相关文件',
    createSuccess: '将初始化项目文件结构',
    createFailed: '创建失败',
    deleteSuccess: '删除成功',
    deleteFailed: '删除失败',
    updatedAt: '更新于',
    status: {
      brainstorm: '思路',
      outline: '大纲',
      ppt: 'PPT'
    },
    noProjects: '暂无项目，点击上方按钮创建'
  },
  brainstorm: {
    title: '思路',
    subtitle: '通过对话厘清你的演示目标和核心信息，AI将帮助你梳理思路并生成需求文档。',
    send: '发送',
    generateRequirements: '生成需求',
    goToOutline: '进入 大纲',
    backToProject: '返回 项目',
    inputPlaceholder: '输入你的问题或想法...',
    thinking: '思考中...',
    emptyChat: '开始对话吧！AI将帮助你厘清演示的核心目标和关键信息。',
    user: '你',
    assistant: 'AI助手',
    uploadAttachment: '上传附件',
    noConversation: '请先进行对话',
    chatFailed: 'AI对话失败',
    generateFailed: '生成需求失败',
    attachments: '附件',
    noAttachments: '暂无附件',
    openFolder: '打开附件文件夹'
  },
  outline: {
    title: '大纲规划',
    promptColumn: '我想要',
    outlineColumn: '大纲',
    style: '风格',
    framework: '框架',
    freeStyle: '自由发挥',
    simple: '简洁',
    medium: '标准',
    detailed: '详细',
    infoDensity: '信息密度',
    expectedPages: '期望页数',
    generateOutline: '生成大纲',
    saveOutline: '保存大纲',
    splitPages: '生成PPT页面',
    backToBrainstorm: '返回 思路',
    goToPages: '进入 PPT设计',
    enterPrompt: '请输入提示词',
    outlinePlaceholder: '生成或输入大纲内容...',
    pagesMustBePositive: '期望页面数必须大于0',
    generateOrEnterOutline: '请先生成或输入大纲内容',
    confirmSplit: '确定要将大纲切分为页面文件吗？',
    saveFailed: '保存失败',
    loadFailed: '加载失败',
    defaultPrompt: `【目标】在此描述演示的核心目标
- 主要目的：说服/教育/汇报/激励
- 期望听众的反应或行动
- 成功的标准

【受众】在此描述目标听众
- 听众背景、知识水平
- 关注点和痛点
- 可能的疑虑

【场合】在此描述演示场景
- 演示场合（会议/演讲/远程）
- 时间限制
- 品牌或视觉要求`
  },
  pages: {
    title: 'PPT设计',
    template: '模板',
    style: '风格',
    freeStyle: '自由发挥',
    backToOutline: '返回 大纲',
    generateImage: '生成当前',
    regenerateImage: '重新生成',
    savePage: '保存页面',
    previousPage: '上一页',
    nextPage: '下一页',
    noImages: '暂无图片',
    generateAll: '批量生成',
    exportPdf: '导出PDF',
    exportPptx: '导出PPT',
    exportEditablePptx: '导出可编辑PPT',
    exportEditablePptxEnhanced: '导出增强版PPT',
    exportTemplatePptx: '基于模板导出',
    selectTemplateAndGenerateFirst: '请先选择模板并生成图片',
    openFolderFailed: '打开文件夹失败',
    keepOnePage: '至少需要保留一个页面',
    deletePageFailed: '删除页面失败',
    selectStyleFirst: '请先选择风格',
    generateImageFailed: '图片生成失败',
    checkErrorLog: '详细信息请查看错误日志',
    batchGenerateFailed: '批量生成失败',
    generateImageFirst: '请先生成图片',
    enterRefinePrompt: '请输入微调要求',
    refineFailed: '图片微调失败',
    exportFailed: '导出失败',
    deleteCurrent: '删除当前',
    imagePreview: '图片预览',
    arrowKeyHint: '方向键翻页',
    refine: '微调',
    openFolder: '打开文件夹'
  },
  skeleton: {
    title: '结构管理',
    create: '新建结构',
    edit: '编辑结构',
    delete: '删除结构',
    name: '结构名称',
    content: '结构内容',
    deleteConfirm: '确定要删除此结构吗？',
    noSkeletons: '暂无结构，点击上方按钮创建',
    loadFailed: '加载结构内容失败',
    enterName: '请输入结构名称',
    saveFailed: '保存失败',
    deleteFailed: '删除失败',
    pageDesc: '结构是一段 Markdown 文本，描述演示的逻辑框架，用于指导 AI 生成大纲。',
    noDescription: '暂无描述',
    defaultTemplate: `# 结构名称

简要描述这个汇报结构的用途和适用场景。

## 核心逻辑

描述这个结构的核心逻辑流程，例如：A → B → C → D

## 适用场景

- 场景1
- 场景2
- 场景3

## 结构要点

1. 第一步：说明
2. 第二步：说明
3. 第三步：说明

## 优势

- 优势1
- 优势2

## 劣势

- 劣势1
- 劣势2

## 示例

提供一个具体的应用示例。
`
  },
  template: {
    title: '模板管理',
    create: '新建模板',
    edit: '编辑模板',
    delete: '删除模板',
    name: '模板名称',
    description: '模板描述',
    deleteConfirm: '确定要删除此模板吗？',
    noTemplates: '暂无模板，请在 templates 文件夹中添加模板',
    coverImage: '封面',
    contentImage: '内容',
    backImage: '封底',
    defaultCannotDelete: '默认模板不能删除',
    deleteFailed: '删除失败',
    deleteNotImplemented: '删除模板功能待实现',
    loadFailed: '加载模板失败',
    pageDesc: '模板包含封面、内容页、封底三张图片，用于指导 AI 生成信息图表的风格和布局。',
    none: '无',
    howToAdd: '如何添加模板',
    helpText1: '在 ./templates/ 文件夹中创建新的文件夹即可添加模板，文件夹名称即为模板名称。',
    helpText2: '每个模板文件夹应包含以下文件：',
    tip: '提示',
    tipDesc: '添加模板文件夹后，点击上方【刷新】按钮即可看到新模板。'
  },
  style: {
    title: '风格管理',
    create: '新建风格',
    edit: '编辑风格',
    delete: '删除风格',
    name: '风格名称',
    content: '风格描述',
    deleteConfirm: '确定要删除此风格吗？',
    noStyles: '暂无风格，点击上方按钮创建',
    extractFromFile: '从文件提取',
    extractFromTemplate: '从模板提取',
    selectTemplate: '选择模板',
    extract: '提取风格',
    loadFailed: '加载风格内容失败',
    enterName: '请输入风格名称',
    saveFailed: '保存失败',
    selectTemplateFirst: '请选择模板',
    extractFailed: '提取风格失败',
    selectFileFirst: '请先选择文件',
    deleteFailed: '删除失败',
    pageDesc: '风格是一段 Markdown 文本，生成图片时会作为提示词的一部分传递给 AI，影响图片的配色、风格等。',
    noColors: '暂无配色信息',
    contentPlaceholder: '描述这个风格的特点，包括配色方案、设计要点等...',
    contentHint: '内容将作为图片生成提示词的一部分，建议包含配色方案、设计风格等描述',
    extractMode: '提取方式',
    selectFile: '选择文件',
    selectFilePlaceholder: '选择图片或PPTX文件',
    browse: '浏览',
    fileHint: '支持 PNG、JPG、WEBP、PPTX 格式。PPTX文件会自动提取配色方案。',
    templateHint: '选择模板后，将自动提取模板的配色方案和风格',
    templatePreview: '模板预览',
    noPreview: '暂无预览图',
    namePlaceholder: '风格名称将与模板名称保持一致',
    extractNote: '提取说明',
    extractNoteFileDesc: '图片提取需要配置支持多模态的 LLM（如 GPT-4o、Qwen-VL 等）。PPTX文件会先提取配色方案，再通过LLM生成风格描述。',
    extractNoteTemplateDesc: '从模板提取会分析模板的配色方案和设计风格，自动生成风格描述。',
    structureIncomplete: '风格结构不完整，缺少以下必填部分：',
    saveAnyway: '是否仍要保存？'
  },
  config: {
    title: 'API 配置',
    description: '配置 LLM 和图像生成 API，用于生成大纲和图片。',
    endpoint: 'API 端点',
    apiKey: 'API 密钥',
    model: '模型名称',
    extraHeaders: '额外请求头',
    addHeader: '添加请求头',
    key: '键',
    value: '值',
    saveSuccess: '配置保存成功',
    saveFailed: '配置保存失败',
    loadFailed: '配置加载失败',
    connectionTest: '测试连接',
    testSuccess: '连接成功',
    testFailed: '连接失败',
    llmConfig: 'LLM 配置（大纲生成）',
    imgConfig: '图像生成配置',
    provider: '提供商',
    customProvider: '自定义',
    enterApiKey: '请输入 API Key',
    enterEndpoint: '请输入 API 端点',
    llmTestSuccess: 'LLM API 连接成功！配置已自动保存',
    llmTestFailed: 'LLM API 连接失败，请检查配置',
    imgTestSuccess: '图像 API 连接成功！配置已自动保存',
    imgTestFailed: '图像 API 连接失败，请检查配置',
    customEndpointHint: '自定义 API 端点，需兼容 OpenAI API 格式',
    customImgEndpointHint: '自定义图像生成 API 端点',
    customHeadersHint: '添加自定义 HTTP 请求头',
    customHeadersHintOptional: '添加自定义 HTTP 请求头（如需要）',
    autoFillEditable: '自动填充，可修改',
    pleaseConfigure: '请先配置 API'
  },
  logs: {
    title: '错误日志',
    clear: '清空日志',
    clearConfirm: '确定要清空所有日志吗？',
    noLogs: '暂无错误日志',
    timestamp: '时间',
    message: '错误信息',
    clearFailed: '清空日志失败',
    loadFailed: '无法加载日志文件',
    description: '查看 error.log 文件内容，用于诊断问题。',
    autoRefresh: '自动刷新',
    stopAutoRefresh: '停止自动刷新',
    scrollToTop: '滚动到顶部',
    scrollToBottom: '滚动到底部'
  },
  imageCard: {
    pageNum: '第 {num} 页',
    pending: '等待生成',
    generating: '生成中...',
    done: '已完成',
    failed: '生成失败',
    refine: '微调',
    regenerate: '重新生成'
  },
  promptEdit: {
    title: '编辑第 {num} 页提示词',
    label: '图片生成提示词',
    placeholder: '描述你想要生成的图片内容...',
    tip: '修改提示词后会重新生成图片。',
    processing: '处理中...',
    regenerate: '重新生成'
  },
  about: {
    title: '关于',
    description: '一款面向专业职场人的、支持私有化模型与品牌资产沉淀的 AI 结构化 PPT 引擎。',
    coreValue: '核心价值',
    coreValueDesc: '我们不解决"从零开始做PPT"的问题，而是解决"从素材到专业演示文稿的最后一公里"：',
    logicStructure: '逻辑梳理与内容结构化',
    logicStructureDesc: 'AI 作为"逻辑架构师"，用户投喂素材和目标，AI 负责生成带有层级的大纲',
    brandCompliance: '企业品牌合规',
    brandComplianceDesc: 'AI 作为"品牌合规守门员"，生成时自动符合企业 VI 规范',
    dataPrivacy: '数据隐私安全',
    dataPrivacyDesc: '支持用户自带 API Key（BYOK），数据在用户的 API 调用链路中流转',
    targetUsers: '目标用户',
    targetUsersList: [
      '企业战略/咨询顾问：每天输出大量汇报材料',
      '售前解决方案专家：频繁输出几十页材料',
      '高校教授/科研人员：拥有大量现成文档',
      '金融分析师：对数据准确性要求极高'
    ],
    differentiation: '差异化优势',
    diffList: [
      '白盒结构化：先定大纲框架，再生成内容，全程可控',
      '深度母版解析：完美还原企业专属 PPTX 母版',
      '原生高保真导出：100% 可编辑的文本框与原生图表',
      'BYOK（自带密钥）：数据隐私安全，模型随心切换'
    ],
    features: '功能特性',
    brainstorming: '思路梳理',
    brainstormingDesc: '通过对话厘清演示目标，上传文本附件，AI 帮助生成需求文档',
    outlineGeneration: '大纲生成',
    outlineGenerationDesc: '智能大纲生成、支持多种汇报框架、信息密度选择',
    pptDesign: 'PPT 设计',
    pptDesignDesc: '批量生成图片、图片微调、模板参考、风格定制',
    exportShare: '导出分享',
    exportShareDesc: 'PDF 导出、PPT 导出',
    customConfig: '自定义配置',
    customConfigDesc: '大模型 API、模板管理、风格管理、框架管理',
    contact: '联系方式',
    github: 'GitHub'
  }
}
