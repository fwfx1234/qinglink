$ErrorActionPreference = 'Stop'

$root = Join-Path $PSScriptRoot '..'
$assetDir = Join-Path $root 'apps\desktop\assets\input'
$serviceExe = 'QingLinkService.exe'

Write-Host '=== 构建安全输入服务 ===' -ForegroundColor Cyan
Push-Location (Join-Path $root 'services\qinglink-service')
cargo build --release
if ($LASTEXITCODE -ne 0) { throw '安全输入服务构建失败' }
Pop-Location

Write-Host '=== 复制服务到资源目录 ===' -ForegroundColor Cyan
if (-not (Test-Path $assetDir)) {
    New-Item -ItemType Directory -Path $assetDir | Out-Null
}
$source = Join-Path (Join-Path $root 'target\release') $serviceExe
$target = Join-Path $assetDir $serviceExe
Copy-Item -Path $source -Destination $target -Force
Write-Host "已复制: $target" -ForegroundColor Green

Write-Host '=== 检查签名 ===' -ForegroundColor Cyan
$signature = Get-AuthenticodeSignature -LiteralPath $target
if ($signature.Status -ne 'Valid') {
    Write-Host "安全输入服务未签名，UAC 将显示未知发布者: $($signature.Status)" -ForegroundColor Yellow
    Write-Host '建议: 使用 Authenticode 签名后重新构建' -ForegroundColor Yellow
} else {
    Write-Host "签名有效: $($signature.SignerCertificate.Subject)" -ForegroundColor Green
}

Write-Host '=== 构建桌面端（嵌入资源）===' -ForegroundColor Cyan
$env:QINGLINK_REQUIRE_INPUT_ASSETS = '1'
cargo build --release -p qinglink-desktop --bin QingLink
if ($LASTEXITCODE -ne 0) { throw '桌面端构建失败' }

$desktopExe = Join-Path (Join-Path $root 'target\release') 'QingLink.exe'
Write-Host ''
Write-Host '=== 构建完成 ===' -ForegroundColor Green
Write-Host "输出文件: $desktopExe"
Write-Host "服务文件: $target"
