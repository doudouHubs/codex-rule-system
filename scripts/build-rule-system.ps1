Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$crateManifest = Join-Path $repoRoot "tools\rule-system\Cargo.toml"
$releaseExe = Join-Path $repoRoot "tools\rule-system\target\release\rule-system.exe"
$binDir = Join-Path $repoRoot "bin"
$targetExe = Join-Path $binDir "rule-system.exe"
$oldPickerExe = Join-Path $binDir "rule-picker-win.exe"

cargo build --release --manifest-path $crateManifest
New-Item -ItemType Directory -Force -Path $binDir | Out-Null
if (Test-Path -LiteralPath $oldPickerExe) {
    # v0.3 起 `rule-system.exe` 是唯一运行时入口，避免旧 picker exe 继续作为幽灵入口被误用。
    Remove-Item -LiteralPath $oldPickerExe -Force
}
Copy-Item -LiteralPath $releaseExe -Destination $targetExe -Force
Write-Host "Built $targetExe"
