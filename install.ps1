# Notidium installer for Windows (PowerShell 5.1+).
#
# Usage (interactive):
#   iwr -useb https://raw.githubusercontent.com/pjankiewicz/notidium/main/install.ps1 | iex
#
# Non-interactive:
#   $env:NOTIDIUM_VAULT = "C:\Users\alice\Notidium"
#   $env:NOTIDIUM_INSTALL_SERVICE = "1"
#   iwr -useb .../install.ps1 | iex
#
# Environment variables:
#   NOTIDIUM_VAULT             Vault path (skips prompt).
#   NOTIDIUM_INSTALL_SERVICE   0 or 1 (skips prompt).
#   NOTIDIUM_NO_MODIFY_PATH    If set, do not modify user PATH.
#   NOTIDIUM_VERSION           Pin to a specific release tag (default: latest).
#   NOTIDIUM_REPO              owner/repo to download from (default: pjankiewicz/notidium).

# Write-Host is intentional: this is an interactive installer whose whole job
# is to print colored progress to the user's terminal. Write-Output/Verbose
# wouldn't give colored, unpiped output — so suppress PSAvoidUsingWriteHost
# at the file level.
[Diagnostics.CodeAnalysis.SuppressMessageAttribute(
    'PSAvoidUsingWriteHost', '',
    Justification = 'Interactive installer requires colored terminal output.'
)]
param()

$ErrorActionPreference = 'Stop'

function Write-Info($msg) {
    Write-Host "==> " -ForegroundColor Cyan -NoNewline
    Write-Host $msg
}

function Write-Err($msg) {
    Write-Host "error: " -ForegroundColor Red -NoNewline
    Write-Host $msg
    exit 1
}

$Repo = if ($env:NOTIDIUM_REPO) { $env:NOTIDIUM_REPO } else { 'pjankiewicz/notidium' }
$InstallDir = Join-Path $env:LOCALAPPDATA 'Notidium\bin'

# ---------- detect target ----------
$arch = (Get-CimInstance Win32_Processor | Select-Object -First 1).Architecture
switch ($arch) {
    9  { $Target = 'x86_64-pc-windows-msvc' }   # AMD64
    12 { Write-Err "ARM64 Windows is not yet supported in CI builds" }
    default { Write-Err "unsupported architecture code: $arch" }
}
Write-Info "Detected target: $Target"

# ---------- resolve release ----------
if ($env:NOTIDIUM_VERSION) {
    $Tag = $env:NOTIDIUM_VERSION
    if (-not $Tag.StartsWith('v')) { $Tag = "v$Tag" }
} else {
    Write-Info "Looking up latest release..."
    try {
        $resp = Invoke-WebRequest -Uri "https://github.com/$Repo/releases/latest" `
            -MaximumRedirection 0 -ErrorAction SilentlyContinue
    } catch {
        $resp = $_.Exception.Response
    }
    $finalUrl = if ($resp.Headers.Location) { $resp.Headers.Location } else { $resp.Headers['Location'] }
    if (-not $finalUrl) {
        # Newer PowerShell returns a full response; follow redirect and read from it.
        $r = Invoke-WebRequest -Uri "https://github.com/$Repo/releases/latest" -MaximumRedirection 5
        $finalUrl = $r.BaseResponse.RequestMessage.RequestUri.AbsoluteUri
    }
    if ($finalUrl -match '/tag/(v[^/]+)/?$') {
        $Tag = $Matches[1]
    } else {
        Write-Err "could not resolve latest release tag from $finalUrl"
    }
}
$Version = $Tag.TrimStart('v')
Write-Info "Installing $Tag"

$Asset = "notidium-$Version-$Target.zip"
$Url = "https://github.com/$Repo/releases/download/$Tag/$Asset"
$SumsUrl = "https://github.com/$Repo/releases/download/$Tag/SHA256SUMS"

$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("notidium-install-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $Tmp | Out-Null

try {
    $ZipPath = Join-Path $Tmp $Asset
    $SumsPath = Join-Path $Tmp 'SHA256SUMS'

    Write-Info "Downloading $Asset..."
    Invoke-WebRequest -Uri $Url -OutFile $ZipPath
    Invoke-WebRequest -Uri $SumsUrl -OutFile $SumsPath

    # ---------- verify checksum ----------
    $sumLine = (Get-Content $SumsPath) | Where-Object { $_ -match "  $([regex]::Escape($Asset))$" } | Select-Object -First 1
    if (-not $sumLine) { Write-Err "checksum for $Asset not found in SHA256SUMS" }
    $expected = ($sumLine -split '\s+')[0].ToLower()
    $actual = (Get-FileHash -Algorithm SHA256 $ZipPath).Hash.ToLower()
    if ($expected -ne $actual) {
        Write-Err "checksum mismatch: expected $expected, got $actual"
    }
    Write-Info "Checksum OK"

    # ---------- extract ----------
    $ExtractDir = Join-Path $Tmp 'extract'
    Expand-Archive -Path $ZipPath -DestinationPath $ExtractDir -Force
    $BinSrc = Join-Path $ExtractDir "notidium-$Version-$Target\notidium.exe"
    if (-not (Test-Path $BinSrc)) {
        Write-Err "archive did not contain notidium.exe at expected path"
    }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -Force $BinSrc (Join-Path $InstallDir 'notidium.exe')
    Write-Info "Installed binary: $InstallDir\notidium.exe"
}
finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}

# ---------- onboarding prompts ----------
function Read-Default($prompt, $default) {
    $ans = Read-Host -Prompt "$prompt [$default]"
    if ([string]::IsNullOrWhiteSpace($ans)) { $default } else { $ans }
}

if ($env:NOTIDIUM_VAULT) {
    $Vault = $env:NOTIDIUM_VAULT
} else {
    $DefaultVault = Join-Path $env:USERPROFILE 'Notidium'
    $Vault = Read-Default 'Vault path' $DefaultVault
}

if ($env:NOTIDIUM_INSTALL_SERVICE) {
    $SetupService = $env:NOTIDIUM_INSTALL_SERVICE -in @('1', 'true', 'yes', 'Y', 'y')
} else {
    $ans = Read-Default 'Set up auto-start at login? (Y/n)' 'Y'
    $SetupService = -not ($ans -in @('n', 'N', 'no', 'NO'))
}

# ---------- initialize vault ----------
$NotidiumExe = Join-Path $InstallDir 'notidium.exe'
if (-not (Test-Path (Join-Path $Vault '.notidium'))) {
    Write-Info "Initializing vault at $Vault"
    & $NotidiumExe init $Vault | Out-Null
}

# ---------- install service ----------
if ($SetupService) {
    Write-Info "Installing auto-start task"
    & $NotidiumExe install-service --vault $Vault --force
}

# ---------- modify user PATH ----------
if (-not $env:NOTIDIUM_NO_MODIFY_PATH) {
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not ($userPath -split ';' | Where-Object { $_ -eq $InstallDir })) {
        $newPath = if ([string]::IsNullOrEmpty($userPath)) { $InstallDir } else { "$InstallDir;$userPath" }
        [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
        Write-Info "Added $InstallDir to user PATH (restart your shell to pick it up)"
    }
}

# ---------- next steps ----------
Write-Host ""
Write-Host "Notidium installed successfully" -ForegroundColor Green
Write-Host ""
Write-Host "  Binary: $NotidiumExe"
Write-Host "  Vault:  $Vault"
if ($SetupService) {
    Write-Host "  Server: http://localhost:3939 (auto-starts at login)"
    Write-Host "  Logs:   $Vault\.notidium\logs\server.log"
} else {
    Write-Host "  Start manually: notidium serve `"$Vault`""
}
