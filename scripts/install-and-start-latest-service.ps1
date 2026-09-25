$ErrorActionPreference = 'Stop'

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw '请以管理员身份运行此脚本。'
}

$serviceName = 'QingLinkService'
$source = Join-Path $PSScriptRoot '..\target\release\QingLinkService.exe' | Resolve-Path
$installDir = Join-Path ${env:ProgramFiles} 'QingLink\Input'
$destination = Join-Path $installDir 'QingLinkService.exe'

if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
    throw "找不到最新构建: $source"
}
New-Item -ItemType Directory -Path $installDir -Force | Out-Null
sc.exe stop $serviceName | Out-Null
Start-Sleep -Seconds 2
Copy-Item -LiteralPath $source -Destination $destination -Force
sc.exe config $serviceName binPath= "`"$destination`"" | Out-Null
sc.exe start $serviceName | Out-Null
Start-Sleep -Seconds 3

$service = Get-CimInstance Win32_Service -Filter "Name = '$serviceName'"
if ($service.State -ne 'Running' -or $service.PathName -notmatch [regex]::Escape($destination)) {
    throw "QingLinkService 启动失败: 状态=$($service.State), 路径=$($service.PathName)"
}
Write-Host "已启动最新 QingLinkService: $destination"
