$ErrorActionPreference = "Stop"

$Version = "1.0.1"
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$StagingDir = Join-Path $PSScriptRoot "staging"
$OutputMsi = Join-Path $PSScriptRoot "FastRGB-Setup-$Version.msi"

# Remove older MSI files so there is only one unambiguous latest installer
Get-ChildItem -Path $PSScriptRoot -Filter "FastRGB-Setup-*.msi" -ErrorAction SilentlyContinue | Remove-Item -Force

Write-Host "[1/3] Building FastRGB v$Version release binary..."
Push-Location $RepoRoot
try {
    cargo build --release -p fastrgb-app
    if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed" }
} finally {
    Pop-Location
}

Write-Host "[2/3] Staging installer payload..."
New-Item -ItemType Directory -Force -Path $StagingDir | Out-Null
Copy-Item (Join-Path $RepoRoot "target\release\fastrgb.exe") (Join-Path $StagingDir "fastrgb.exe") -Force
Copy-Item (Join-Path $RepoRoot "assets\fastrgb.ico") (Join-Path $StagingDir "fastrgb.ico") -Force

$InpOutCandidates = @(
    (Join-Path $RepoRoot "target\release\inpoutx64.dll"),
    (Join-Path $RepoRoot "target\debug\inpoutx64.dll")
)
$InpOutSrc = $InpOutCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $InpOutSrc) {
    throw "inpoutx64.dll not found in target/release or target/debug"
}
Copy-Item $InpOutSrc (Join-Path $StagingDir "inpoutx64.dll") -Force

Write-Host "[3/3] Building Windows MSI via WiX Toolset..."
Push-Location $PSScriptRoot
try {
    wix build -arch x64 -o $OutputMsi FastRGB.wxs
    if ($LASTEXITCODE -ne 0) { throw "wix build failed" }
} finally {
    Pop-Location
}

$Info = Get-Item $OutputMsi
Write-Host "Built installer: $($Info.FullName) ($([math]::Round($Info.Length / 1MB, 2)) MB)"
