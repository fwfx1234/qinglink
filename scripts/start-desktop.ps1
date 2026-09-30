param(
    [switch]$SkipBuild,
    [string]$DataDir
)

$ErrorActionPreference = 'Stop'

# 构建并可靠地启动桌面端。
#
# 直接以管道方式启动会让父 shell 等待子进程，且父作业被回收时子进程会一起
# 被杀掉（表现为“启动后进程消失 / 托盘图标闪一下就不见了”）。这里改用
# Start-Process 并把 stdout/stderr 重定向到文件，启动后立即返回。

$root = Join-Path $PSScriptRoot '..'

if (-not $SkipBuild) {
    Write-Host '=== 构建桌面端 ===' -ForegroundColor Cyan
    Push-Location $root
    cargo build -p qinglink-desktop
    if ($LASTEXITCODE -ne 0) {
        Pop-Location
        throw '桌面端构建失败'
    }
    Pop-Location
}

$exe = Join-Path $root 'target\debug\QingLink.exe'
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    throw "找不到桌面端可执行文件: $exe"
}

if (-not $DataDir) {
    $DataDir = Join-Path $root 'target\qinglink-runtime'
}
New-Item -ItemType Directory -Path $DataDir -Force | Out-Null
$logDir = Join-Path $DataDir 'logs'
New-Item -ItemType Directory -Path $logDir -Force | Out-Null

$stdout = Join-Path $logDir 'desktop.stdout.log'
$stderr = Join-Path $logDir 'desktop.stderr.log'

$env:QINGLINK_DATA_DIR = $DataDir
$process = Start-Process -FilePath $exe -WorkingDirectory $root -RedirectStandardOutput $stdout -RedirectStandardError $stderr -PassThru

# 启动较慢（数据库迁移 + 服务生命周期），这里只确认进程已创建。
Start-Sleep -Seconds 2
$running = Get-Process -Id $process.Id -ErrorAction SilentlyContinue
if (-not $running) {
    Write-Host "桌面端启动后立即退出，请查看日志:" -ForegroundColor Yellow
    Write-Host "  $stderr"
    exit 1
}

Write-Host "已启动 QingLink Desktop: PID $($process.Id)" -ForegroundColor Green
Write-Host "数据目录: $DataDir"
Write-Host "日志: $logDir"
Write-Host '提示: 托盘图标可能在启动约 20 秒后出现；Windows 11 默认收进“^”隐藏图标区。'
