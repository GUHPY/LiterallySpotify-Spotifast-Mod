param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
Set-Location -LiteralPath $root
$metadata = (& cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).packages |
    Where-Object { $_.name -eq 'spotifast' } | Select-Object -First 1
if ($LASTEXITCODE -ne 0 -or !$metadata) { throw 'Unable to read Cargo metadata.' }
$version = $metadata.version
if ($metadata.repository -ne 'https://github.com/GUHPY/LiterallySpotify-Spotifast-Mod') { throw 'This script only packages the LiterallySpotify fork.' }
if (!$SkipBuild) {
    & cargo build --release --locked --bin Spotify -j 1
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
}
$targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'target' }
$binary = Join-Path $targetRoot 'release/Spotify.exe'
if (!(Test-Path -LiteralPath $binary)) { throw 'Spotify.exe is missing.' }
$reported = (& $binary --version | Out-String).Trim()
if ($reported -ne "literallyspotify $version") { throw "Unexpected binary identity: $reported" }
$name = "literallyspotify-v$version-x86_64-pc-windows-msvc"
$dist = Join-Path $root 'dist'
$stage = Join-Path $dist $name
New-Item -ItemType Directory -Path $stage -Force | Out-Null
Copy-Item -LiteralPath $binary -Destination (Join-Path $stage 'Spotify.exe') -Force
foreach ($file in @('README.md','LICENSE')) { Copy-Item -LiteralPath (Join-Path $root $file) -Destination (Join-Path $stage $file) -Force }
'literallyspotify-portable-v1' | Set-Content -LiteralPath (Join-Path $stage 'literallyspotify-portable.txt') -Encoding ascii
Compress-Archive -LiteralPath $stage -DestinationPath (Join-Path $dist "$name.zip") -Force
$iscc = Get-Command ISCC.exe -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -First 1
if (!$iscc) {
    $candidates = @(
        (Join-Path $env:LOCALAPPDATA 'Programs/Inno Setup 6/ISCC.exe'),
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6/ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 6/ISCC.exe')
    )
    $iscc = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if (!$iscc) { throw 'Install Inno Setup 6 to build the Windows installer.' }
& $iscc "/DVersion=$version" '/DArch=x86_64' "/DBinary=$binary" "/DOutputDir=$dist" 'packaging/windows/spotifast.iss'
if ($LASTEXITCODE -ne 0) { throw 'Inno Setup failed.' }
Move-Item -LiteralPath (Join-Path $dist 'Spotify-Setup.exe') -Destination (Join-Path $dist "$name-setup.exe") -Force
Get-FileHash -LiteralPath $binary,(Join-Path $dist "$name.zip"),(Join-Path $dist "$name-setup.exe") -Algorithm SHA256
