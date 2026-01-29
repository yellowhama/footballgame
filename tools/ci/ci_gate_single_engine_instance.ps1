param(
  [string]$RepoRoot = ".",
  [string]$AllowFile = "autoload/rust/FootballRustEngine.gd"
)

$ErrorActionPreference = "Stop"

function Write-Fail([string]$Code, [string]$Message) {
  Write-Host "[FAIL] $Code"
  Write-Host $Message
  exit 1
}

Set-Location $RepoRoot

$pattern = 'ClassDB.instantiate("FootballMatchSimulator")'
$normalizedAllow = $AllowFile.Replace("\", "/")

function Normalize-RelPath([string]$Path) {
  $p = $Path
  if ($p.StartsWith(".\")) { $p = $p.Substring(2) }
  if ($p.StartsWith("./")) { $p = $p.Substring(2) }
  return $p.Replace("\", "/")
}

$rg = Get-Command rg -ErrorAction SilentlyContinue
if (-not $rg) {
  Write-Fail "E_RG_NOT_FOUND" "ripgrep (rg) not found; install rg or implement a fallback search."
}

$hits = & rg -n -F $pattern -g "*.gd" -g "*.tscn" . 2>$null
if (-not $hits) {
  Write-Host "[OK] no forbidden instantiation found"
  exit 0
}

$bad = @()
foreach ($line in $hits) {
  # Format: path:line:match
  if ($line -match '^(?<path>[^:]+):(?<lineno>\d+):') {
    $path = Normalize-RelPath $Matches["path"]
    if ($path -ne $normalizedAllow) {
      $bad += $line
    }
  } else {
    $bad += $line
  }
}

if ($bad.Count -gt 0) {
  Write-Fail "E_SINGLETON_ENGINE_INSTANTIATION_FORBIDDEN" ("FootballMatchSimulator는 `"$AllowFile`"에서만 생성 가능. 직접 instantiate 금지.`n`n" + ($bad -join "`n"))
}

Write-Host "[OK] instantiation only in $AllowFile"
exit 0
