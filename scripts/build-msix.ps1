<#
  build-msix.ps1 - lay out and pack TidyUp as an MSIX for the Microsoft Store.
  Compact port of Spaceadom's script. ASCII ONLY in this file (Windows PowerShell 5.1
  reads a .ps1 without BOM as ANSI).

  Never installs anything. The package is unsigned unless -Sign (self-signed, local
  structural test only; the Store re-signs on ingestion).

  Usage:  npm run build ; npm run tauri build ; npm run msix [-- -Sign]
  Output: src-tauri/target/release/bundle/msix/TidyUp_<version>_x64.msix
  Exit:   0 packed and validated, 1 error, 2 Windows SDK (MakeAppx) missing.
#>
[CmdletBinding()]
param([switch]$Sign, [switch]$SkipValidate, [ValidateSet('x64','arm64')][string]$Arch = 'x64')
$ErrorActionPreference = 'Stop'
function Say ($m) { Write-Host "build-msix: $m" }
function Die ($m) { Write-Host "build-msix: ERROR - $m" -ForegroundColor Red; exit 1 }
function Warn ($m) { Write-Host "build-msix: WARNING - $m" -ForegroundColor Yellow }

$Root    = Split-Path -Parent $PSScriptRoot
$MsixSrc = Join-Path $Root 'src-tauri\msix'
$Layout  = Join-Path $MsixSrc 'layout'
$Assets  = Join-Path $Layout 'Assets'
$OutDir  = Join-Path $Root 'src-tauri\target\release\bundle\msix'
$RelDir  = if ($Arch -eq 'arm64') { Join-Path $Root 'src-tauri\target\aarch64-pc-windows-msvc\release' } else { Join-Path $Root 'src-tauri\target\release' }

# 1. version from package.json -> a.b.c.0
$version3 = (Get-Content (Join-Path $Root 'package.json') -Raw | ConvertFrom-Json).version
if ($version3 -notmatch '^\d+\.\d+\.\d+$') { Die "package.json version '$version3' is not a.b.c" }
$version4 = "$version3.0"
Say "version $version4, arch $Arch"

# 2. identity
$identityPath = Join-Path $MsixSrc 'identity.json'
if (-not (Test-Path $identityPath)) { Die "src-tauri/msix/identity.json is missing. Copy identity.example.json and fill it from Partner Center > TidyUp > Product identity." }
$identity = Get-Content $identityPath -Raw | ConvertFrom-Json
foreach ($f in 'name','publisher','publisherDisplayName') {
  if ([string]::IsNullOrWhiteSpace($identity.$f) -or $identity.$f -like '*PUT-*') { Die "identity.json '$f' is still a placeholder" }
}
if ($identity.publisher -notmatch '^CN=') { Die "identity.json 'publisher' must start with CN=" }
Say "identity: $($identity.name) / $($identity.publisherDisplayName)"

# 3. the release binary
$exe = Join-Path $RelDir 'tidy.exe'
if (-not (Test-Path $exe)) { Die "no release binary at $exe - run 'npm run tauri build' first" }
$exeVer = (Get-Item $exe).VersionInfo.FileVersion
if ($exeVer -and ($exeVer -notlike "$version3*")) { Die "tidy.exe reports FileVersion '$exeVer' but package.json says '$version3' - rebuild first" }
Say ("binary: {0:N0} bytes, FileVersion {1}" -f (Get-Item $exe).Length, $exeVer)

# 4. layout: exe + logos + manifest
try { Add-Type -AssemblyName System.Drawing -ErrorAction Stop } catch { Add-Type -AssemblyName System.Drawing.Common -ErrorAction Stop }
function New-Logo([string]$Src, [string]$Dest, [int]$W, [int]$H) {
  $img = [System.Drawing.Image]::FromFile($Src)
  try {
    $bmp = New-Object System.Drawing.Bitmap($W, $H, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    try {
      $g = [System.Drawing.Graphics]::FromImage($bmp)
      try {
        $g.InterpolationMode = 'HighQualityBicubic'; $g.PixelOffsetMode = 'HighQuality'; $g.SmoothingMode = 'HighQuality'
        $g.Clear([System.Drawing.Color]::Transparent)
        $scale = [Math]::Min($W / $img.Width, $H / $img.Height)
        $dw = [int][Math]::Round($img.Width * $scale); $dh = [int][Math]::Round($img.Height * $scale)
        $g.DrawImage($img, [int](($W - $dw) / 2), [int](($H - $dh) / 2), $dw, $dh)
      } finally { $g.Dispose() }
      $bmp.Save($Dest, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally { $bmp.Dispose() }
  } finally { $img.Dispose() }
}
if (Test-Path $Layout) { Remove-Item $Layout -Recurse -Force }
New-Item -ItemType Directory -Force -Path $Assets | Out-Null
$iconSrc = Join-Path $Root 'src-tauri\icons\icon.png'
if (-not (Test-Path $iconSrc)) { Die "no icon at $iconSrc" }
$logos = @(
  @{ Name = 'Square44x44Logo.png'; W = 44; H = 44 }, @{ Name = 'Square71x71Logo.png'; W = 71; H = 71 },
  @{ Name = 'Square150x150Logo.png'; W = 150; H = 150 }, @{ Name = 'Square310x310Logo.png'; W = 310; H = 310 },
  @{ Name = 'Wide310x150Logo.png'; W = 310; H = 150 }, @{ Name = 'StoreLogo.png'; W = 50; H = 50 })
foreach ($l in $logos) { New-Logo $iconSrc (Join-Path $Assets $l.Name) $l.W $l.H }
Copy-Item $exe (Join-Path $Layout 'tidy.exe') -Force

$manifest = [IO.File]::ReadAllText((Join-Path $MsixSrc 'AppxManifest.xml'))
$manifest = $manifest.Replace('{{IDENTITY_NAME}}', $identity.name).Replace('{{IDENTITY_PUBLISHER}}', $identity.publisher).Replace('{{PUBLISHER_DISPLAY_NAME}}', $identity.publisherDisplayName).Replace('{{VERSION}}', $version4).Replace('{{ARCH}}', $Arch)
if ($manifest -match '\{\{[A-Z_]+\}\}') { Die "placeholder survived: $($Matches[0])" }
[IO.File]::WriteAllText((Join-Path $Layout 'AppxManifest.xml'), $manifest, (New-Object Text.UTF8Encoding($false)))

# 5. TaskId contract check against packaged.rs
$rs = Get-Content (Join-Path $Root 'src-tauri\src\packaged.rs') -Raw
$m = [regex]::Match($rs, 'STARTUP_TASK_ID:\s*&(?:''static\s+)?str\s*=\s*"([^"]+)"')
if (-not $m.Success) { Die "STARTUP_TASK_ID not found in packaged.rs" }
$xmlTaskId = ([xml]$manifest).Package.Applications.Application.Extensions.Extension.StartupTask.TaskId
if ($m.Groups[1].Value -ne $xmlTaskId) { Die "TaskId mismatch: manifest '$xmlTaskId' vs packaged.rs '$($m.Groups[1].Value)'" }
Say "startupTask id '$xmlTaskId' matches packaged.rs"

# 6. pack
function Find-SdkTool([string]$Name) {
  $bin = 'C:\Program Files (x86)\Windows Kits\10\bin'
  if (-not (Test-Path $bin)) { return $null }
  Get-ChildItem $bin -Directory -ErrorAction SilentlyContinue | Where-Object { $_.Name -match '^10\.' } | Sort-Object { [version]($_.Name) } -Descending | ForEach-Object { Join-Path $_.FullName "x64\$Name" } | Where-Object { Test-Path $_ } | Select-Object -First 1
}
$makeappx = Find-SdkTool 'makeappx.exe'
if (-not $makeappx) { Warn "MakeAppx.exe not found. Layout is complete at $Layout. Install the Windows SDK and re-run."; exit 2 }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$msix = Join-Path $OutDir "TidyUp_${version3}_${Arch}.msix"
if (Test-Path $msix) { Remove-Item $msix -Force }
& $makeappx pack /d $Layout /p $msix /o
if ($LASTEXITCODE -ne 0) { Die "makeappx pack failed ($LASTEXITCODE)" }
Say ("packed: {0} ({1:N1} MB)" -f $msix, ((Get-Item $msix).Length / 1MB))

if ($Sign) {
  $signtool = Find-SdkTool 'signtool.exe'
  if (-not $signtool) { Warn "SignTool.exe not found - package left unsigned" }
  else {
    $pfx = Join-Path $MsixSrc 'test-signing.pfx'; $pw = 'tidyup-local-test'
    if (-not (Test-Path $pfx)) {
      $cert = New-SelfSignedCertificate -Type Custom -KeyUsage DigitalSignature -CertStoreLocation 'Cert:\CurrentUser\My' -Subject $identity.publisher -FriendlyName 'TidyUp LOCAL MSIX TEST - not for distribution' -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')
      Export-PfxCertificate -Cert "Cert:\CurrentUser\My\$($cert.Thumbprint)" -FilePath $pfx -Password (ConvertTo-SecureString $pw -Force -AsPlainText) | Out-Null
    }
    & $signtool sign /fd SHA256 /a /f $pfx /p $pw $msix
    if ($LASTEXITCODE -ne 0) { Warn "signtool failed ($LASTEXITCODE) - package left unsigned" } else { Say "signed with the LOCAL TEST certificate" }
  }
} else { Say "NOT signed (pass -Sign for a local test signature)" }

# 7. validate: unpack and compare every file byte for byte
if ($SkipValidate) { exit 0 }
$unpacked = Join-Path $MsixSrc 'unpacked'
if (Test-Path $unpacked) { Remove-Item $unpacked -Recurse -Force }
& $makeappx unpack /p $msix /d $unpacked /o
if ($LASTEXITCODE -ne 0) { Die "makeappx unpack failed ($LASTEXITCODE)" }
$problems = @()
foreach ($f in (Get-ChildItem $Layout -Recurse -File | ForEach-Object { $_.FullName.Substring($Layout.Length + 1) })) {
  $u = Join-Path $unpacked $f
  if (-not (Test-Path $u)) { $problems += "missing: $f"; continue }
  if ((Get-FileHash (Join-Path $Layout $f)).Hash -ne (Get-FileHash $u).Hash) { $problems += "changed: $f" }
}
if ($problems) { $problems | ForEach-Object { Write-Host "  $_" }; Die "round-trip validation failed" }
Say "validated: every layout file came back byte-identical"
exit 0
