$ErrorActionPreference = 'Stop'

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw '请以管理员身份运行此脚本。'
}

$serviceName = 'QingqiVirtualInput'
$service = Get-CimInstance Win32_Service -Filter "Name = '$serviceName'" -ErrorAction SilentlyContinue
if ($service) {
    if ($service.State -eq 'Running') {
        sc.exe stop $serviceName | Out-Null
        $deadline = (Get-Date).AddSeconds(15)
        do {
            Start-Sleep -Milliseconds 250
            $service = Get-CimInstance Win32_Service -Filter "Name = '$serviceName'" -ErrorAction SilentlyContinue
        } while ($service -and $service.State -eq 'Running' -and (Get-Date) -lt $deadline)
    }
    sc.exe delete $serviceName | Out-Null
}

$legacyExe = 'F:\develop\qingqi-workspace\qingqi\target\release\qingqi-input-service.exe'
if (Test-Path -LiteralPath $legacyExe -PathType Leaf) {
    Remove-Item -LiteralPath $legacyExe -Force
}

$driverOutput = @(pnputil.exe /enum-drivers)
$published = $null
for ($index = 0; $index -lt $driverOutput.Count; $index++) {
    if ($driverOutput[$index] -match 'Original Name\s*:\s*qingqi_virtual_keyboard\.inf') {
        for ($previous = $index; $previous -ge 0; $previous--) {
            if ($driverOutput[$previous] -match 'Published Name\s*:\s*(oem\d+\.inf)') {
                $published = $Matches[1]
                break
            }
        }
        break
    }
}
if ($published) {
    pnputil.exe /delete-driver $published /uninstall /force | Out-Null
}

if (Get-CimInstance Win32_Service -Filter "Name = '$serviceName'" -ErrorAction SilentlyContinue) {
    throw "$serviceName 服务仍然存在。"
}
if (Get-CimInstance Win32_Process -Filter "Name = 'qingqi-input-service.exe'" -ErrorAction SilentlyContinue) {
    throw 'qingqi-input-service.exe 进程仍在运行。'
}
Write-Host 'QingqiVirtualInput 及其遗留输入组件已清理。'
