$ErrorActionPreference = 'Stop'
$dir = Join-Path $env:TEMP 'dsh-parts'
New-Item -ItemType Directory -Force -Path $dir | Out-Null
Get-ChildItem $dir -File -ErrorAction SilentlyContinue | Remove-Item -Force
$url = 'https://github.com/anywhere-labs/deepseek-harness-desktop/releases/download/v2.0.13/DSH-Desktop-2.0.13-x64-Setup.exe'
$total = 156056465
$count = 12
$partSize = [math]::Ceiling($total / $count)
$procs = @()
for ($i = 0; $i -lt $count; $i++) {
  $start = $i * $partSize
  $end = [math]::Min($total - 1, (($i + 1) * $partSize) - 1)
  $part = Join-Path $dir ("part-{0:D2}" -f $i)
  $arguments = @('-x', 'http://127.0.0.1:7891', '-L', '--fail', '--retry', '5', '--range', "$start-$end", '-o', $part, $url)
  $procs += Start-Process -FilePath 'curl.exe' -ArgumentList $arguments -PassThru -WindowStyle Hidden
}
while ($true) {
  $running = @($procs | Where-Object { -not $_.HasExited })
  $bytes = (Get-ChildItem $dir -File -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum
  Write-Output ("running={0} bytes={1} of {2}" -f $running.Count, $bytes, $total)
  if ($running.Count -eq 0) { break }
  Start-Sleep -Seconds 5
}
if (($procs | Where-Object { $_.ExitCode -ne 0 }).Count -gt 0) { throw 'One or more download parts failed.' }
$output = Join-Path $env:TEMP 'DSH-Desktop-2.0.13-x64-Setup.exe'
$out = [System.IO.File]::Open($output, [System.IO.FileMode]::Create)
try {
  for ($i = 0; $i -lt $count; $i++) {
    $part = Join-Path $dir ("part-{0:D2}" -f $i)
    $in = [System.IO.File]::OpenRead($part)
    try { $in.CopyTo($out) } finally { $in.Dispose() }
  }
} finally { $out.Dispose() }
Get-FileHash $output -Algorithm SHA256
Get-Item $output | Select-Object FullName, Length
