param([string]$Version = '6.8.3')
$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'This installer targets Windows x64 MSVC. Use a native Qt SDK on other platforms.' }
$taskRoot = Split-Path $PSScriptRoot -Parent
$taskVenv = Join-Path $taskRoot '.tools/qt-installer'
$taskPython = Join-Path $taskVenv 'Scripts/python.exe'
if (-not (Test-Path -LiteralPath $taskPython)) {
    & python -m venv $taskVenv
    if ($LASTEXITCODE -ne 0) { throw 'Python venv creation failed.' }
}
& $taskPython -m pip install 'aqtinstall==3.3.0'
if ($LASTEXITCODE -ne 0) { throw 'aqtinstall installation failed.' }
$taskSdk = Join-Path $taskRoot ".tools/qt/$Version/msvc2022_64"
if (-not (Test-Path -LiteralPath (Join-Path $taskSdk 'lib/cmake/Qt6/Qt6Config.cmake'))) {
    & $taskPython -m aqt install-qt windows desktop $Version win64_msvc2022_64 -O (Join-Path $taskRoot '.tools/qt') --archives qtbase qtdeclarative
    if ($LASTEXITCODE -ne 0) { throw 'Qt SDK installation failed.' }
}
Write-Output "Qt SDK: $taskSdk"
Write-Output "Configure with: cmake -S . -B build/qt -DDRAWVERSE_BUILD_UI=ON -DCMAKE_PREFIX_PATH=`"$taskSdk`" -DCMAKE_BUILD_TYPE=Release"
