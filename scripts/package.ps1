# PptBuilder 一键打包脚本
# 1. 编译 Vue 前端
# 2. 编译 Tauri 后端
# 3. 打包为 ZIP（包含 exe + config + templates + styles + skeletons + models + projects）

$ErrorActionPreference = "Stop"

$ROOT_DIR = Split-Path -Parent $PSScriptRoot
$SRC_TAURI_DIR = Join-Path $ROOT_DIR "src-tauri"
$RELEASE_DIR = Join-Path $SRC_TAURI_DIR "target\release"
$BUNDLE_DIR = Join-Path $RELEASE_DIR "bundle"
$OUTPUT_DIR = Join-Path $ROOT_DIR "output"
$VERSION = "0.2.0"

Write-Host "🚀 PptBuilder 打包脚本" -ForegroundColor Green
Write-Host "======================"
Write-Host "根目录: $ROOT_DIR"
Write-Host ""

# Step 1: 编译 Vue 前端
Write-Host "📦 Step 1: 编译 Vue 前端..." -ForegroundColor Cyan
Set-Location $ROOT_DIR
npm run build
if ($LASTEXITCODE -ne 0) {
    Write-Host "⚠️ TypeScript 检查失败，尝试跳过 TS 检查..." -ForegroundColor Yellow
    npm run build:skip-ts
    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Vue 前端编译失败" -ForegroundColor Red
        exit 1
    }
    Write-Host "✅ Vue 前端编译完成（跳过 TS 检查）" -ForegroundColor Green
} else {
    Write-Host "✅ Vue 前端编译完成" -ForegroundColor Green
}

# Step 2: 编译 Tauri 后端
Write-Host ""
Write-Host "🔧 Step 2: 编译 Tauri 后端..." -ForegroundColor Cyan
npm run tauri:build
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Tauri 后端编译失败" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Tauri 后端编译完成" -ForegroundColor Green

# Step 3: 准备输出目录
Write-Host ""
Write-Host "📁 Step 3: 准备输出目录..." -ForegroundColor Cyan

# 清理旧输出
if (Test-Path $OUTPUT_DIR) {
    Remove-Item -Recurse -Force $OUTPUT_DIR
}
New-Item -ItemType Directory -Path $OUTPUT_DIR -Force | Out-Null

# 创建打包目录
$PACKAGE_NAME = "PptBuilder_${VERSION}_win64"
$PACKAGE_DIR = Join-Path $OUTPUT_DIR $PACKAGE_NAME
New-Item -ItemType Directory -Path $PACKAGE_DIR -Force | Out-Null

# Step 4: 复制可执行文件
Write-Host "📋 Step 4: 复制可执行文件..." -ForegroundColor Cyan

# 复制 MSI
$msiPath = Join-Path $BUNDLE_DIR "msi\PptBuilder_${VERSION}_x64_en-US.msi"
if (Test-Path $msiPath) {
    Copy-Item $msiPath $OUTPUT_DIR
    Write-Host "  ✅ 复制 MSI: PptBuilder_${VERSION}_x64_en-US.msi" -ForegroundColor Green
}

# 复制 EXE 安装包
$exePath = Join-Path $BUNDLE_DIR "nsis\PptBuilder_${VERSION}_x64-setup.exe"
if (Test-Path $exePath) {
    Copy-Item $exePath $OUTPUT_DIR
    Write-Host "  ✅ 复制 EXE: PptBuilder_${VERSION}_x64-setup.exe" -ForegroundColor Green
}

# 复制便携版 exe
$portableExe = Join-Path $RELEASE_DIR "PptBuilder.exe"
if (Test-Path $portableExe) {
    Copy-Item $portableExe (Join-Path $PACKAGE_DIR "PptBuilder.exe")
    Write-Host "  ✅ 复制便携版: PptBuilder.exe" -ForegroundColor Green
}

# 复制 WebView2Loader.dll
$dllPath = Join-Path $RELEASE_DIR "WebView2Loader.dll"
if (Test-Path $dllPath) {
    Copy-Item $dllPath (Join-Path $PACKAGE_DIR "WebView2Loader.dll")
    Write-Host "  ✅ 复制依赖库: WebView2Loader.dll" -ForegroundColor Green
} else {
    Write-Host "  ⚠️ 警告: 未找到 WebView2Loader.dll" -ForegroundColor Yellow
}

# Step 5: 复制数据文件夹
Write-Host "📋 Step 5: 复制数据文件夹..." -ForegroundColor Cyan

# 从 src-tauri 复制的数据文件夹
$srcTauriDataFolders = @("templates", "styles", "skeletons", "models")
$srcTauriDataFiles = @("config.yaml")
$rootDataFolders = @("projects")

# 从 src-tauri 复制 templates、styles、skeletons、models
foreach ($folder in $srcTauriDataFolders) {
    $srcPath = Join-Path $SRC_TAURI_DIR $folder
    $destPath = Join-Path $PACKAGE_DIR $folder
    
    if (Test-Path $srcPath) {
        Copy-Item -Recurse -Force $srcPath $destPath
        Write-Host "  ✅ 复制: $folder" -ForegroundColor Green
    } else {
        New-Item -ItemType Directory -Path $destPath -Force | Out-Null
        Write-Host "  📁 创建: $folder" -ForegroundColor Yellow
    }
}

# 从 src-tauri 复制配置文件
foreach ($file in $srcTauriDataFiles) {
    $srcPath = Join-Path $SRC_TAURI_DIR $file
    $destPath = Join-Path $PACKAGE_DIR $file
    
    if (Test-Path $srcPath) {
        Copy-Item -Force $srcPath $destPath
        Write-Host "  ✅ 复制: $file" -ForegroundColor Green
    } else {
        # 如果配置文件不存在，创建默认配置
        $defaultConfig = @"
llm:
  provider: openai
  endpoint: https://api.openai.com/v1
  api_key: ""
  model: gpt-4o
  extra_headers: []

img:
  provider: openai
  endpoint: https://api.openai.com/v1/images/generations
  api_key: ""
  model: gpt-image-2
  extra_headers: []

image_sizes:
  - name: "16:9 横屏"
    width: 1920
    height: 1072
"@
        $defaultConfig | Out-File -FilePath $destPath -Encoding utf8
        Write-Host "  📝 创建默认配置: $file" -ForegroundColor Yellow
    }
}

# 从 root 复制其他文件夹
foreach ($folder in $rootDataFolders) {
    $srcPath = Join-Path $ROOT_DIR $folder
    $destPath = Join-Path $PACKAGE_DIR $folder
    
    if (Test-Path $srcPath) {
        Copy-Item -Recurse -Force $srcPath $destPath
        Write-Host "  ✅ 复制: $folder" -ForegroundColor Green
    } else {
        New-Item -ItemType Directory -Path $destPath -Force | Out-Null
        Write-Host "  📁 创建: $folder" -ForegroundColor Yellow
    }
}

# Step 6: 创建 ZIP 包
Write-Host ""
Write-Host "📦 Step 6: 创建 ZIP 包..." -ForegroundColor Cyan

$zipPath = Join-Path $OUTPUT_DIR "${PACKAGE_NAME}.zip"
Compress-Archive -Path $PACKAGE_DIR -DestinationPath $zipPath -Force
Write-Host "✅ ZIP 包已创建: $zipPath" -ForegroundColor Green

# 完成
Write-Host ""
Write-Host "🎉 打包完成！" -ForegroundColor Green
Write-Host "📁 输出目录: $OUTPUT_DIR"
Write-Host "📦 安装包目录: $PACKAGE_DIR"
Write-Host "📦 ZIP 文件: $zipPath"

# 打开输出目录
Start-Process "explorer.exe" $OUTPUT_DIR
