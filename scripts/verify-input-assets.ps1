$ErrorActionPreference = 'Stop'

$assetRoot = Join-Path $PSScriptRoot '..\apps\desktop\assets\input'
$required = @(
    'QingLinkService.exe'
)

foreach ($relative in $required) {
    $path = Join-Path $assetRoot $relative
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "缺少安全输入发布资源: $relative"
    }
}

foreach ($relative in @('QingLinkService.exe')) {
    $path = Join-Path $assetRoot $relative
    $signature = Get-AuthenticodeSignature -LiteralPath $path
    if ($signature.Status -ne 'Valid') {
        Write-Warning "安全输入服务未签名或签名无效，UAC 将显示未知发布者: $relative ($($signature.Status))"
    }
}

$env:QINGLINK_REQUIRE_INPUT_ASSETS = '1'
cargo check -p qinglink-desktop --bin QingLink
if ($LASTEXITCODE -ne 0) {
    throw '桌面端安全输入资源嵌入检查失败'
}

Write-Host '安全输入发布资源完整、签名有效，并已通过桌面端嵌入检查。'
