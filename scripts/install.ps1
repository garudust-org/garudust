<#
.SYNOPSIS
  Garudust installer for Windows (x86_64).

.EXAMPLE
  irm https://raw.githubusercontent.com/garudust-org/garudust-agent/main/scripts/install.ps1 | iex

.NOTES
  Overrides via environment variables:
    GARUDUST_VERSION   pin a release tag (e.g. v0.13.1); default: latest
    GARUDUST_BIN_DIR   install destination; default: %LOCALAPPDATA%\Programs\garudust
#>

$ErrorActionPreference = 'Stop'
$repo = 'garudust-org/garudust-agent'

function Info($m) { Write-Host "==> $m" -ForegroundColor Cyan }
function Die($m)  { Write-Host "error: $m" -ForegroundColor Red; exit 1 }

# ── Detect architecture ──────────────────────────────────────────────────────
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -ne 'AMD64') {
  Die "unsupported architecture '$arch'. Only x86_64 Windows binaries are published."
}
$target = 'x86_64-pc-windows-msvc'

# ── Resolve version ──────────────────────────────────────────────────────────
$version = $env:GARUDUST_VERSION
if (-not $version) {
  Info 'Resolving latest release...'
  $rel = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
  $version = $rel.tag_name
  if (-not $version) { Die 'could not determine the latest version. Set GARUDUST_VERSION.' }
}

$asset = "garudust-$version-$target.zip"
$url   = "https://github.com/$repo/releases/download/$version/$asset"

# ── Download ─────────────────────────────────────────────────────────────────
$tmp = Join-Path $env:TEMP ("garudust-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Info "Downloading $asset..."
  $zip = Join-Path $tmp $asset
  Invoke-WebRequest -Uri $url -OutFile $zip

  # ── Verify checksum (best-effort) ──────────────────────────────────────────
  try {
    $sums = (Invoke-WebRequest "https://github.com/$repo/releases/download/$version/SHA256SUMS.txt").Content
    $line = ($sums -split "`n" | Where-Object { $_ -match [regex]::Escape($asset) } | Select-Object -First 1)
    if ($line) {
      $expected = ($line -split '\s+')[0]
      $actual   = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
      if ($actual -ne $expected.ToLower()) { Die "checksum mismatch for $asset." }
      Info 'Checksum verified.'
    }
  } catch { Write-Host "warning: skipping checksum verification." -ForegroundColor Yellow }

  # ── Extract ──────────────────────────────────────────────────────────────────
  Expand-Archive -Path $zip -DestinationPath $tmp -Force
  $src = Join-Path $tmp "garudust-$version-$target"

  # ── Install ──────────────────────────────────────────────────────────────────
  $binDir = $env:GARUDUST_BIN_DIR
  if (-not $binDir) { $binDir = Join-Path $env:LOCALAPPDATA 'Programs\garudust' }
  New-Item -ItemType Directory -Path $binDir -Force | Out-Null

  foreach ($b in 'garudust.exe', 'garudust-server.exe') {
    Copy-Item (Join-Path $src $b) (Join-Path $binDir $b) -Force
  }
  Info "Installed garudust $version -> $binDir"

  # ── PATH ─────────────────────────────────────────────────────────────────────
  $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  if ($userPath -notlike "*$binDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$binDir", 'User')
    Write-Host "Added $binDir to your user PATH. Restart your shell to pick it up." -ForegroundColor Yellow
  }
  Info "Run 'garudust setup' to get started."
}
finally {
  Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
