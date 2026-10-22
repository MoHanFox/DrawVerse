param(
    [string]$RuntimeDirectory = (Join-Path $PSScriptRoot '../build/bin/Release'),
    [string]$OutputDirectory = (Join-Path $PSScriptRoot ('../artifacts/issue-2-main-b1/student-' + (Get-Date -Format 'yyyyMMdd-HHmmss-fff'))),
    [ValidateSet('auto','wintab','windows-ink')][string]$TabletApi = 'auto'
)
$ErrorActionPreference = 'Stop'
$runtime = (Resolve-Path -LiteralPath $RuntimeDirectory).Path
$exe = Join-Path $runtime 'drawverse.exe'
foreach ($entry in @('drawverse.exe','Qt6Core.dll','Qt6Quick.dll','platforms/qwindows.dll','qml/QtQuick')) {
    if (!(Test-Path -LiteralPath (Join-Path $runtime $entry))) { throw "Incomplete deployed runtime: $entry" }
}
# No force, cleanup, or reuse: keep every previous student session intact.
$output = New-Item -ItemType Directory -Path $OutputDirectory -ErrorAction Stop
$profile = New-Item -ItemType Directory -Path (Join-Path $output.FullName 'profile')
$temporary = New-Item -ItemType Directory -Path (Join-Path $output.FullName 'temp')
@('[input]',"tabletApi=$TabletApi") | Set-Content -LiteralPath (Join-Path $profile.FullName 'DrawVerse.ini') -Encoding utf8
$digest = [System.Security.Cryptography.SHA256]::Create()
$stream = [System.IO.File]::OpenRead($exe)
try { $hash = [BitConverter]::ToString($digest.ComputeHash($stream)).Replace('-','') }
finally { $stream.Dispose(); $digest.Dispose() }
@{Algorithm='SHA256'; Hash=$hash; Path=$exe} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $output.FullName 'binary.json')
$previous = @{}
foreach ($name in @('TEMP','TMP','QT_QPA_PLATFORM','QT_QUICK_BACKEND','QT_FORCE_STDERR_LOGGING','QSG_RHI_BACKEND','QSG_INFO','QSG_RHI_DISABLE_DISK_CACHE','QML_DISK_CACHE_PATH')) { $previous[$name] = [Environment]::GetEnvironmentVariable($name,'Process') }
try {
    $env:TEMP = $temporary.FullName; $env:TMP = $temporary.FullName
    $env:QT_QPA_PLATFORM = 'windows'; $env:QSG_RHI_BACKEND = 'd3d11'; $env:QT_QUICK_BACKEND = $null
    $env:QT_FORCE_STDERR_LOGGING = '1'
    $env:QSG_INFO = '1'; $env:QSG_RHI_DISABLE_DISK_CACHE = '1'
    $env:QML_DISK_CACHE_PATH = Join-Path $output.FullName 'qml-cache'
    Write-Host "Isolated session: $($output.FullName)"
    # This is the requested interactive native window, with the complete DLL/QML deployment.
    $process = Start-Process -FilePath $exe -ArgumentList @('--profile-dir',('"'+$profile.FullName+'"')) -WorkingDirectory $runtime -WindowStyle Normal -PassThru -Wait -RedirectStandardOutput (Join-Path $output.FullName 'stdout.log') -RedirectStandardError (Join-Path $output.FullName 'stderr.log')
    if ($process.ExitCode -ne 0) { throw "DrawVerse exited with $($process.ExitCode); see session logs" }
} finally {
    foreach ($name in $previous.Keys) { [Environment]::SetEnvironmentVariable($name,$previous[$name],'Process') }
}
