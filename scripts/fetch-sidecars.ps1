# scripts/fetch-sidecars.ps1
# Downloads prebuilt whisper.cpp + llama.cpp Windows binaries from official releases.
# Run once before first `tauri build` (or `tauri dev`).
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$dest = Join-Path $root "src-tauri\sidecar"
New-Item -ItemType Directory -Force -Path $dest | Out-Null

# Whisper.cpp
$whisperVer = "v1.7.2"
$whisperBin = "whisper-bin-x64.zip"
$whisperUrl = "https://github.com/ggerganov/whisper.cpp/releases/download/$whisperVer/$whisperBin"
Write-Host "Downloading whisper.cpp $whisperVer..."
try {
    Invoke-WebRequest -Uri $whisperUrl -OutFile "$dest\$whisperBin" -UseBasicParsing
} catch {
    Write-Warning "Whisper download failed (network or version unavailable): $_"
    Write-Warning "You can manually place whisper-server.exe in $dest"
}
if (Test-Path "$dest\Release\whisper-server.exe") {
    Move-Item -Path "$dest\Release\whisper-server.exe" -Destination "$dest\whisper-server.exe" -Force
}
if (Test-Path "$dest\$whisperBin") { Remove-Item "$dest\$whisperBin" -Force }
if (Test-Path "$dest\Release") { Remove-Item -Recurse -Force "$dest\Release" }

# Llama.cpp (skip if download fails; user can fetch manually)
$llamaVer = "b5103"
$llamaBin = "llama-$llamaVer-bin-win-cuda12.4-x64.zip"
$llamaUrl = "https://github.com/ggerganov/llama.cpp/releases/download/$llamaVer/$llamaBin"
Write-Host "Downloading llama.cpp $llamaVer..."
try {
    Invoke-WebRequest -Uri $llamaUrl -OutFile "$dest\$llamaBin" -UseBasicParsing -TimeoutSec 120
    Expand-Archive -Path "$dest\$llamaBin" -DestinationPath "$dest\llama-tmp" -Force
    $llamaExe = Get-ChildItem "$dest\llama-tmp" -Filter "llama-server.exe" -Recurse | Select-Object -First 1
    if ($llamaExe) {
        Move-Item -Path $llamaExe.FullName -Destination "$dest\llama-server.exe" -Force
    }
    Remove-Item -Recurse -Force "$dest\llama-tmp", "$dest\$llamaBin"
} catch {
    Write-Warning "Llama download failed: $_"
    Write-Warning "You can manually place llama-server.exe in $dest"
}

Write-Host "Done. Sidecar binaries in $dest:"
Get-ChildItem $dest -Filter "*.exe"