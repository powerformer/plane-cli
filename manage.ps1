$ErrorActionPreference = 'Stop'

$command = if ($args.Length -gt 0) { $args[0] } else { 'install' }
[string[]]$remaining = if ($args.Length -gt 1) { $args[1..($args.Length - 1)] } else { @() }
$pathCommand = ''
if ($command -eq 'path') {
    $pathCommand = if ($remaining.Length -gt 0) { $remaining[0] } else { 'help' }
    [string[]]$remaining = if ($remaining.Length -gt 1) { $remaining[1..($remaining.Length - 1)] } else { @() }
}

$channel = if ($env:PLANE_CHANNEL) { $env:PLANE_CHANNEL } else { 'stable' }
$version = if ($env:PLANE_VERSION) { $env:PLANE_VERSION } else { '' }
$publicUrl = if ($env:PLANE_RELEASES_PUBLIC_URL) { $env:PLANE_RELEASES_PUBLIC_URL } else { 'https://releases.plane.powerformer.net' }
$defaultInstallRoot = if ($env:LOCALAPPDATA) { Join-Path $env:LOCALAPPDATA 'plane' } else { Join-Path $HOME '.local/share/plane' }
$defaultLocalBinDir = if ($env:USERPROFILE) { Join-Path $env:USERPROFILE '.local\bin' } else { Join-Path $HOME '.local/bin' }
$installRoot = if ($env:PLANE_INSTALL_ROOT) { $env:PLANE_INSTALL_ROOT } else { $defaultInstallRoot }
$localBinDir = if ($env:PLANE_LOCAL_BIN_DIR) { $env:PLANE_LOCAL_BIN_DIR } else { $defaultLocalBinDir }
$retain = if ($env:PLANE_RETAIN) { $env:PLANE_RETAIN } else { '' }
$pathMarkerStart = '# >>> plane-cli path >>>'
$pathMarkerEnd = '# <<< plane-cli path <<<'

function Show-Help {
    @'
plane manager

Usage:
  manage.ps1 install [--channel stable|beta] [--version vX.Y.Z] [--retain[=true|false]]
  manage.ps1 upgrade [--channel stable|beta] [--version vX.Y.Z] [--retain[=true|false]]
  manage.ps1 uninstall [--version vX.Y.Z]
  manage.ps1 path setup
  manage.ps1 path clear

Environment:
  PLANE_RELEASES_PUBLIC_URL  # default: https://releases.plane.powerformer.net
  PLANE_CHANNEL
  PLANE_VERSION
  PLANE_INSTALL_ROOT
  PLANE_LOCAL_BIN_DIR
  PLANE_RETAIN
'@ | Write-Output
}

function Show-PathHelp {
    @'
plane manager path commands

Usage:
  manage.ps1 path setup [--bin-dir DIR]
  manage.ps1 path clear

`path setup` appends a managed PATH block to the current-user PowerShell profile.
`path clear` removes only the managed block marked by plane-cli comments.
'@ | Write-Output
}

if ($command -match '^(-h|--help|help)$') {
    Show-Help
    exit 0
}

for ($i = 0; $i -lt $remaining.Length; $i++) {
    $arg = $remaining[$i]
    switch -Regex ($arg) {
        '^--channel$' { $i++; $channel = $remaining[$i]; continue }
        '^--channel=(.+)$' { $channel = $Matches[1]; continue }
        '^--version$' { $i++; $version = $remaining[$i]; continue }
        '^--version=(.+)$' { $version = $Matches[1]; continue }
        '^--public-url$' { $i++; $publicUrl = $remaining[$i]; continue }
        '^--public-url=(.+)$' { $publicUrl = $Matches[1]; continue }
        '^--install-root$' { $i++; $installRoot = $remaining[$i]; continue }
        '^--install-root=(.+)$' { $installRoot = $Matches[1]; continue }
        '^--bin-dir$' { $i++; $localBinDir = $remaining[$i]; continue }
        '^--bin-dir=(.+)$' { $localBinDir = $Matches[1]; continue }
        '^--retain$' { $retain = 'true'; continue }
        '^--retain=(.+)$' { $retain = $Matches[1]; continue }
        '^(-h|--help|help)$' {
            if ($command -eq 'path') {
                Show-PathHelp
            }
            else {
                Show-Help
            }
            exit 0
        }
        default { throw "unknown argument: $arg" }
    }
}

function Normalize-Version {
    param([string]$Value)
    return "v$($Value.TrimStart('v'))"
}

function Normalize-Bool {
    param([string]$Value)
    switch -Regex ($Value) {
        '^(true|1|yes|y|on)$' { return $true }
        '^(false|0|no|n|off)$' { return $false }
        default { throw "invalid --retain value: $Value" }
    }
}

function Quote-PSLiteral {
    param([string]$Value)
    return "'$($Value.Replace("'", "''"))'"
}

function Get-PlaneCommandPath {
    return Join-Path $localBinDir 'plane.cmd'
}

function Get-PlaneShimPaths {
    return @(
        (Join-Path $localBinDir 'plane.cmd'),
        (Join-Path $localBinDir 'plane.ps1'),
        (Join-Path $localBinDir 'plane.exe')
    )
}

function Write-PlaneShim {
    param([string]$VersionRoot)
    $exePath = Join-Path $VersionRoot 'plane.exe'
    New-Item -ItemType Directory -Force -Path $localBinDir | Out-Null
    Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $localBinDir 'plane.exe')

    $cmdPath = Join-Path $localBinDir 'plane.cmd'
    $cmdContent = "@echo off`r`n`"$exePath`" %*`r`n"
    Set-Content -LiteralPath $cmdPath -Value $cmdContent -NoNewline -Encoding ASCII

    $ps1Path = Join-Path $localBinDir 'plane.ps1'
    $ps1Content = "& $(Quote-PSLiteral $exePath) @args`nexit `$LASTEXITCODE`n"
    Set-Content -LiteralPath $ps1Path -Value $ps1Content -NoNewline -Encoding UTF8
}

function Test-PlaneCommandResolvesToLocalBin {
    $command = Get-Command plane -ErrorAction SilentlyContinue | Select-Object -First 1
    if (!$command) {
        return $false
    }
    $source = if ($command.Source) { $command.Source } else { $command.Path }
    if ([string]::IsNullOrWhiteSpace($source)) {
        return $false
    }
    $expected = @(Get-PlaneShimPaths | ForEach-Object { [System.IO.Path]::GetFullPath($_) })
    return $expected -contains [System.IO.Path]::GetFullPath($source)
}

function Show-PathNoticeIfNeeded {
    if (Test-PlaneCommandResolvesToLocalBin) {
        return
    }
    [Console]::Error.WriteLine("plane: plane does not resolve to $(Get-PlaneCommandPath) in this shell.")
    [Console]::Error.WriteLine("temporary: `$env:Path = `"$localBinDir$([System.IO.Path]::PathSeparator)`$env:Path`"")
    [Console]::Error.WriteLine("persist: .\manage.ps1 path setup")
}

function Path-SetupBlock {
    if ($localBinDir -eq $defaultLocalBinDir) {
        $line = "`$planeCliBin = Join-Path `$HOME '.local\bin'"
    }
    else {
        $line = "`$planeCliBin = $(Quote-PSLiteral $localBinDir)"
    }
    return @"
$pathMarkerStart
$line
if ((`$env:Path -split [System.IO.Path]::PathSeparator) -notcontains `$planeCliBin) {
    `$env:Path = "`${planeCliBin}$([System.IO.Path]::PathSeparator)`$env:Path"
}
$pathMarkerEnd
"@
}

function Remove-ManagedPathBlock {
    param([string]$Path)
    if (![System.IO.File]::Exists($Path)) {
        return $false
    }
    $content = Get-Content -Raw -LiteralPath $Path
    if ($null -eq $content) {
        $content = ''
    }
    if (!$content.Contains($pathMarkerStart) -or !$content.Contains($pathMarkerEnd)) {
        return $false
    }
    $pattern = "(?ms)^[`t ]*$([regex]::Escape($pathMarkerStart))\r?\n.*?^[`t ]*$([regex]::Escape($pathMarkerEnd))\r?\n?"
    $updated = [regex]::Replace($content, $pattern, '')
    if ($updated -eq $content) {
        return $false
    }
    Set-Content -LiteralPath $Path -Value $updated -NoNewline -Encoding UTF8
    return $true
}

function User-ProfilePaths {
    $paths = @($PROFILE.CurrentUserCurrentHost, $PROFILE.CurrentUserAllHosts)
    return @($paths | Where-Object { ![string]::IsNullOrWhiteSpace($_) } | Select-Object -Unique)
}

function Test-ManagedPathBlock {
    foreach ($path in User-ProfilePaths) {
        if (![System.IO.File]::Exists($path)) {
            continue
        }
        $content = Get-Content -Raw -LiteralPath $path
        if ($null -eq $content) {
            $content = ''
        }
        if ($content.Contains($pathMarkerStart) -and $content.Contains($pathMarkerEnd)) {
            return $true
        }
    }
    return $false
}

function Setup-Path {
    if ($pathCommand -match '^(-h|--help|help)$' -or [string]::IsNullOrWhiteSpace($pathCommand)) {
        Show-PathHelp
        return
    }
    if ($pathCommand -ne 'setup') {
        throw "unknown path command: $pathCommand"
    }

    $profilePath = $PROFILE.CurrentUserCurrentHost
    $profileDir = Split-Path -Parent $profilePath
    New-Item -ItemType Directory -Force -Path $profileDir | Out-Null
    if (![System.IO.File]::Exists($profilePath)) {
        New-Item -ItemType File -Force -Path $profilePath | Out-Null
    }
    $updated = Remove-ManagedPathBlock $profilePath
    Add-Content -LiteralPath $profilePath -Value "`n$(Path-SetupBlock)"
    if ($updated) {
        Write-Output "plane: updated PATH in $profilePath"
    }
    else {
        Write-Output "plane: configured PATH in $profilePath"
    }
}

function Clear-Path {
    if ($pathCommand -match '^(-h|--help|help)$' -or [string]::IsNullOrWhiteSpace($pathCommand)) {
        Show-PathHelp
        return
    }
    if ($pathCommand -ne 'clear') {
        throw "unknown path command: $pathCommand"
    }

    $removed = $false
    foreach ($profilePath in User-ProfilePaths) {
        if (Remove-ManagedPathBlock $profilePath) {
            Write-Output "plane: removed PATH block from $profilePath"
            $removed = $true
        }
    }
    if (!$removed) {
        Write-Output 'plane: no managed PATH block found'
    }
}

function Installed-Versions {
    param([string]$Current)
    if (![System.IO.Directory]::Exists($installRoot)) {
        return @()
    }
    return @(Get-ChildItem -LiteralPath $installRoot -Directory | Where-Object { $_.Name -ne $Current -and $_.Name -match '^v[0-9]+' } | ForEach-Object { $_.Name })
}

function Should-Retain {
    param([string[]]$OldVersions)
    if ($OldVersions.Length -eq 0) {
        return $true
    }
    if (![string]::IsNullOrWhiteSpace($retain)) {
        return Normalize-Bool $retain
    }
    if ([Environment]::UserInteractive -and -not [Console]::IsInputRedirected) {
        $answer = Read-Host 'plane: remove previously installed versions after install? [y/N]'
        if ($answer -match '^(y|yes)$') {
            return $false
        }
        return $true
    }
    [Console]::Error.WriteLine('plane: preserving previous versions; pass --retain=false to prune after install')
    return $true
}

function Install-Plane {
    $resolvedPublicUrl = $publicUrl.TrimEnd('/')
    $resolvedVersion = $version
    if ([string]::IsNullOrWhiteSpace($resolvedVersion)) {
        $metadataUrl = "$resolvedPublicUrl/$channel/latest/metadata.json"
        $metadata = Invoke-RestMethod -Uri $metadataUrl
        $resolvedVersion = $metadata.releaseVersion
        if ([string]::IsNullOrWhiteSpace($resolvedVersion)) {
            throw 'failed to resolve latest plane version'
        }
    }
    $resolvedVersion = Normalize-Version $resolvedVersion
    $script:version = $resolvedVersion
    $oldVersions = Installed-Versions $resolvedVersion
    $retainOld = Should-Retain $oldVersions

    $archive = 'plane-x86_64-pc-windows-msvc.zip'
    $tmpdir = Join-Path ([System.IO.Path]::GetTempPath()) ("plane-" + [System.Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmpdir | Out-Null
    try {
        $archivePath = Join-Path $tmpdir $archive
        Invoke-WebRequest -Uri "$resolvedPublicUrl/$channel/versions/$resolvedVersion/$archive" -OutFile $archivePath
        $versionRoot = Join-Path $installRoot $resolvedVersion
        Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $versionRoot
        New-Item -ItemType Directory -Force -Path $versionRoot | Out-Null
        Expand-Archive -LiteralPath $archivePath -DestinationPath $versionRoot -Force
        Write-PlaneShim $versionRoot
        & (Get-PlaneCommandPath) --version
        Show-PathNoticeIfNeeded

        if (!$retainOld) {
            foreach ($oldVersion in $oldVersions) {
                Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $installRoot $oldVersion)
                Write-Output "removed old plane $oldVersion from $installRoot"
            }
        }

        Write-Output "installed plane to $(Get-PlaneCommandPath)"
    }
    finally {
        Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $tmpdir
    }
}

function Upgrade-Plane {
    Install-Plane
    & (Get-PlaneCommandPath) skill upgrade --channel $channel --version $script:version --release-url $publicUrl
}

function Remove-EmptyDir {
    param([string]$Path)
    if ([System.IO.Directory]::Exists($Path)) {
        try {
            [System.IO.Directory]::Delete($Path, $false)
        }
        catch {}
    }
}

function Installed-Version {
    $binPath = Get-PlaneCommandPath
    if (![System.IO.File]::Exists($binPath)) {
        return ''
    }
    try {
        $output = & $binPath --version
        if ($output -match 'v?([0-9]+\.[0-9]+\.[0-9]+(?:[-.][A-Za-z0-9]+)*)') {
            return "v$($Matches[1].TrimStart('v'))"
        }
    }
    catch {}
    return ''
}

function Uninstall-Plane {
    $binPath = Get-PlaneCommandPath
    if (![string]::IsNullOrWhiteSpace($version)) {
        $normalizedVersion = Normalize-Version $version
        if ((Installed-Version) -eq $normalizedVersion) {
            foreach ($path in Get-PlaneShimPaths) {
                Remove-Item -Force -ErrorAction SilentlyContinue $path
            }
            Write-Output "removed $binPath"
        }
        Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $installRoot $normalizedVersion)
        Remove-EmptyDir $installRoot
        Write-Output "removed plane $normalizedVersion from $installRoot"
        if (Test-ManagedPathBlock) {
            Write-Output 'plane: run `.\manage.ps1 path clear` to remove the managed PATH block'
        }
        return
    }

    foreach ($path in Get-PlaneShimPaths) {
        Remove-Item -Force -ErrorAction SilentlyContinue $path
    }
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $installRoot
    Remove-EmptyDir $localBinDir
    Write-Output "removed plane from $installRoot and $binPath"
    if (Test-ManagedPathBlock) {
        Write-Output 'plane: run `.\manage.ps1 path clear` to remove the managed PATH block'
    }
}

switch ($command) {
    'install' { Install-Plane }
    'upgrade' { Upgrade-Plane }
    'uninstall' { Uninstall-Plane }
    'path' {
        switch ($pathCommand) {
            'setup' { Setup-Path }
            'clear' { Clear-Path }
            { $_ -match '^(-h|--help|help)$' -or [string]::IsNullOrWhiteSpace($_) } { Show-PathHelp }
            default { throw "unknown path command: $pathCommand" }
        }
    }
    default { throw "unknown command: $command" }
}
