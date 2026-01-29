param(
  [string]$RepoRoot = ".",
  [string]$ContractFile = "docs/ssot/BRIDGE_CONTRACT_SSOT.json"
)

$ErrorActionPreference = "Stop"

function Write-Fail([string]$Code, [string]$Message) {
  Write-Host "[FAIL] $Code"
  Write-Host $Message
  exit 1
}

function Should-CheckMethodName([string]$Name) {
  if ($Name -eq "test_connection") { return $true }
  if ($Name -like "simulate_match_*") { return $true }
  if ($Name -in @(
    "start_match_session",
    "step_match_session",
    "finish_match_session",
    "change_live_tactic",
    "change_formation_live_match"
  )) { return $true }
  return $false
}

Set-Location $RepoRoot

if (-not (Test-Path $ContractFile)) {
  Write-Fail "E_BRIDGE_CONTRACT_SSOT_NOT_FOUND" "Contract file not found: $ContractFile"
}

$contract = Get-Content $ContractFile -Raw | ConvertFrom-Json
if (-not $contract.methods) {
  Write-Fail "E_BRIDGE_CONTRACT_SSOT_INVALID" "Contract file missing methods[]: $ContractFile"
}

$allowed = New-Object "System.Collections.Generic.HashSet[string]"
foreach ($m in $contract.methods) { [void]$allowed.Add([string]$m.name) }

$files = New-Object "System.Collections.Generic.HashSet[string]"
foreach ($m in $contract.methods) {
  if ($null -eq $m.used_by) { continue }
  foreach ($f in $m.used_by) { [void]$files.Add([string]$f) }
}

if ($files.Count -eq 0) {
  Write-Fail "E_BRIDGE_CONTRACT_USED_BY_EMPTY" "No used_by files found in $ContractFile"
}

$unknown = @()
$missingFiles = @()

foreach ($file in $files) {
  if (-not (Test-Path $file)) {
    $missingFiles += $file
    continue
  }

  $content = Get-Content $file -Raw

  # 1) String-literal calls: _rust.call("name", ...), has_method("name"), etc
  $strMatches = [regex]::Matches($content, '(?:call|has_method)\(\s*"(?<name>[^"]+)"')
  foreach ($m in $strMatches) {
    $name = $m.Groups["name"].Value
    if (Should-CheckMethodName $name) {
      if (-not $allowed.Contains($name)) {
        $unknown += "${file}: string-call '$name' not in SSOT"
      }
    }
  }

  # 2) Direct calls on likely Rust handles: _rust.<name>(...), _rust_simulator.<name>(...), ...
  $directMatches = [regex]::Matches($content, '(?:_rust|_rust_simulator|_rust_match_simulator|simulator)\.(?<name>[A-Za-z_][A-Za-z0-9_]*)\s*\(')
  foreach ($m in $directMatches) {
    $name = $m.Groups["name"].Value
    if (Should-CheckMethodName $name) {
      if (-not $allowed.Contains($name)) {
        $unknown += "${file}: direct-call '$name' not in SSOT"
      }
    }
  }
}

if ($missingFiles.Count -gt 0) {
  Write-Fail "E_BRIDGE_CONTRACT_USED_BY_MISSING_FILES" ("Missing used_by files referenced by ${ContractFile}:`n" + ($missingFiles -join "`n"))
}

if ($unknown.Count -gt 0) {
  Write-Fail "E_BRIDGE_CONTRACT_USED_BY_DRIFT" ("Found method calls not in SSOT:`n" + ($unknown -join "`n"))
}

Write-Host "[OK] bridge contract used_by matches SSOT ($($allowed.Count) methods; $($files.Count) files)"
exit 0
