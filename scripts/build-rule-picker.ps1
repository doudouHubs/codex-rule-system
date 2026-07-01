Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$crateManifest = Join-Path $repoRoot "tools\rule-picker-win\Cargo.toml"
$releaseExe = Join-Path $repoRoot "tools\rule-picker-win\target\release\rule-picker-win.exe"
$binDir = Join-Path $repoRoot "bin"
$targetExe = Join-Path $binDir "rule-picker-win.exe"

cargo build --release --manifest-path $crateManifest
New-Item -ItemType Directory -Force -Path $binDir | Out-Null
Copy-Item -LiteralPath $releaseExe -Destination $targetExe -Force
Write-Host "Built $targetExe"
