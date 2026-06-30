$ErrorActionPreference = 'Stop'

# Release smoke for the plane-cli meta-management surface on Windows. Mirrors
# smoke.sh: install the command entry and shims, run the installed CLI
# (version/help and a mocked read-only API call), manage agent skills, set up and
# clear the PATH block, and uninstall cleanly. Release artifacts are read from
# R2/S3 and served from a local HTTP mirror so CI never depends on the public
# Cloudflare edge.

$root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))))
$version = if ($args.Length -gt 0) { $args[0] } else { throw 'missing release version' }
$channel = if ($args.Length -gt 1) { $args[1] } else { 'stable' }

$pathMarkerStart = '# >>> plane-cli path >>>'

$tmpdir = Join-Path ([System.IO.Path]::GetTempPath()) ("plane-smoke-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmpdir | Out-Null
$server = $null

function Get-Python {
    foreach ($candidate in @('python3', 'python')) {
        $command = Get-Command $candidate -ErrorAction SilentlyContinue
        if ($command) {
            return $command.Source
        }
    }
    throw 'python3 or python is required for the release smoke mirror'
}

function Get-FreePort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Parse('127.0.0.1'), 0)
    try {
        $listener.Start()
        return $listener.LocalEndpoint.Port
    }
    finally {
        $listener.Stop()
    }
}

function Wait-SmokeMirror {
    param([string]$Url)
    $attempts = if ($env:SMOKE_MIRROR_ATTEMPTS) { [int]$env:SMOKE_MIRROR_ATTEMPTS } else { 50 }
    $delayMs = if ($env:SMOKE_MIRROR_DELAY_MS) { [int]$env:SMOKE_MIRROR_DELAY_MS } else { 200 }
    for ($i = 0; $i -lt $attempts; $i++) {
        try {
            Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri $Url | Out-Null
            return
        }
        catch {
            Start-Sleep -Milliseconds $delayMs
        }
    }
    throw "timed out waiting for smoke mirror: $Url"
}

function Wait-MockApi {
    param([string]$Url, [string]$Key)
    $attempts = if ($env:SMOKE_MIRROR_ATTEMPTS) { [int]$env:SMOKE_MIRROR_ATTEMPTS } else { 50 }
    $delayMs = if ($env:SMOKE_MIRROR_DELAY_MS) { [int]$env:SMOKE_MIRROR_DELAY_MS } else { 200 }
    for ($i = 0; $i -lt $attempts; $i++) {
        try {
            Invoke-WebRequest -UseBasicParsing -TimeoutSec 2 -Uri $Url -Headers @{ 'X-API-Key' = $Key } | Out-Null
            return
        }
        catch {
            Start-Sleep -Milliseconds $delayMs
        }
    }
    throw "timed out waiting for mock api: $Url"
}

function Get-Manage {
    return (Join-Path $root 'manage.ps1')
}

function Get-PlaneCmd {
    return (Join-Path $env:PLANE_LOCAL_BIN_DIR 'plane.cmd')
}

function Capture-ConsoleError {
    param([scriptblock]$Action)
    # manage.ps1 prints the PATH notice via [Console]::Error.WriteLine, which
    # bypasses PowerShell stream redirection. Capture it by swapping the .NET
    # Console error writer for the duration of the action.
    $writer = [System.IO.StringWriter]::new()
    $previous = [Console]::Error
    [Console]::SetError($writer)
    try {
        & $Action | Out-Null
    }
    finally {
        [Console]::SetError($previous)
    }
    return $writer.ToString()
}

function Count-PathMarkers {
    param($ProfileObject)
    $total = 0
    foreach ($path in @($ProfileObject.CurrentUserCurrentHost, $ProfileObject.CurrentUserAllHosts)) {
        if ([System.IO.File]::Exists($path)) {
            $hits = Select-String -SimpleMatch -Pattern $pathMarkerStart -Path $path
            $total += @($hits).Count
        }
    }
    return $total
}

function Smoke-Install {
    & (Get-Manage) install --channel $channel --version $version --retain=false
    $cmd = Join-Path $env:PLANE_LOCAL_BIN_DIR 'plane.cmd'
    $ps1 = Join-Path $env:PLANE_LOCAL_BIN_DIR 'plane.ps1'
    if (!(Test-Path $cmd)) { throw "install missing command entry $cmd" }
    if (!(Test-Path $ps1)) { throw "install missing shim $ps1" }
    $expectedExe = Join-Path (Join-Path $env:PLANE_INSTALL_ROOT $version) 'plane.exe'
    $cmdContent = Get-Content -Raw -LiteralPath $cmd
    if ($cmdContent -notmatch [regex]::Escape($expectedExe)) {
        throw "plane.cmd does not point at the versioned install root: expected $expectedExe"
    }
}

function Smoke-CliBasics {
    $versionOutput = (& (Get-PlaneCmd) --version | Out-String)
    if ($versionOutput -notmatch [regex]::Escape($version)) {
        throw "plane --version did not contain ${version}: $versionOutput"
    }
    & (Get-PlaneCmd) help | Out-Null
}

function Smoke-ApiMeMock {
    # Production Plane is IP-allowlisted (CI runners are not on it) and we never
    # want a real token in CI, so exercise `plane api me` against a local mock of
    # /api/v1/users/me/ that also asserts the X-API-Key header.
    $apiPort = Get-FreePort
    $apiKey = 'smoke-mock-api-key'
    $apiServer = Start-Process -FilePath $python -ArgumentList @(
        (Join-Path $root '.github/scripts/release/smoke/mock_api.py'),
        '--port', "$apiPort", '--key', $apiKey
    ) -PassThru
    try {
        $apiBase = "http://127.0.0.1:$apiPort"
        Wait-MockApi "$apiBase/api/v1/users/me/" $apiKey
        $env:PLANE_API_BASE_URL = $apiBase
        $env:PLANE_API_KEY = $apiKey

        $plane = Get-PlaneCmd
        $apiOutput = (& $plane api me | Out-String)
        if ($apiOutput -notmatch 'Plane API smoke ok') { throw "api me did not report success: $apiOutput" }
        if ($apiOutput -notmatch 'smoke@plane.test') { throw "api me did not render the mock user: $apiOutput" }
        if ($apiOutput -match [regex]::Escape($apiKey)) { throw 'api me output leaked the API token' }
        & $plane api me --json | Out-Null
        Write-Output "api me ok against mock $apiBase"
    }
    finally {
        $env:PLANE_API_BASE_URL = $null
        $env:PLANE_API_KEY = $null
        if ($apiServer -and !$apiServer.HasExited) {
            Stop-Process -Id $apiServer.Id -Force -ErrorAction SilentlyContinue
            $apiServer.WaitForExit()
        }
    }
}

function Smoke-SkillLifecycle {
    $plane = Get-PlaneCmd
    & $plane skill install --path $skillPath --channel $channel --version $version
    & $plane skill list
    if (!(Test-Path (Join-Path $skillPath 'SKILL.md'))) { throw "skill install missing $(Join-Path $skillPath 'SKILL.md')" }
    if (!(Test-Path (Join-Path $skillPath 'metadata.json'))) { throw "skill install missing $(Join-Path $skillPath 'metadata.json')" }
}

function Smoke-SkillContentBoundary {
    $skillMd = Join-Path $skillPath 'SKILL.md'
    if (!(Select-String -SimpleMatch -Pattern 'Version Selection' -Path $skillMd)) {
        throw 'distributed skill is missing the user-facing Version Selection section'
    }
    # Terminal-user-facing only: reject repo operator release/R2/publishing
    # wording. "published version" is allowed, so use word boundaries on publish.
    $forbidden = @('release behavior', 'workflow', '\bR2\b', 'operator', '\bpublish\b', 'publishing', 'runseal')
    foreach ($pattern in $forbidden) {
        if (Select-String -Pattern $pattern -Path $skillMd) {
            throw "distributed skill leaks operator wording matching /$pattern/"
        }
    }
}

function Smoke-Upgrade {
    & (Get-Manage) upgrade --channel $channel --version $version --retain=false
    if (!(Test-Path (Join-Path $skillPath 'SKILL.md'))) { throw "skill upgrade missing $(Join-Path $skillPath 'SKILL.md')" }
    $list = (& (Get-PlaneCmd) skill list | Out-String)
    if ($list -notmatch [regex]::Escape("binary $version")) { throw "skill list did not report binary ${version}: $list" }
    if ($list -notmatch [regex]::Escape("skill $version")) { throw "skill list did not report skill ${version}: $list" }
}

function Smoke-PathSetupClear {
    $profileDir = Join-Path $tmpdir 'pwsh-profile'
    New-Item -ItemType Directory -Force -Path $profileDir | Out-Null
    $isolatedProfile = [pscustomobject]@{
        CurrentUserCurrentHost = Join-Path $profileDir 'Microsoft.PowerShell_profile.ps1'
        CurrentUserAllHosts    = Join-Path $profileDir 'profile.ps1'
    }
    $savedProfile = $PROFILE
    $PROFILE = $isolatedProfile
    try {
        & (Get-Manage) path setup --bin-dir $env:PLANE_LOCAL_BIN_DIR
        & (Get-Manage) path setup --bin-dir $env:PLANE_LOCAL_BIN_DIR
        $count = Count-PathMarkers $isolatedProfile
        if ($count -ne 1) { throw "expected exactly 1 managed PATH marker after setup, found $count" }
        # The generated profile block must parse as valid PowerShell.
        $content = Get-Content -Raw -LiteralPath $isolatedProfile.CurrentUserCurrentHost
        [scriptblock]::Create($content) | Out-Null
        & (Get-Manage) path clear
        $count = Count-PathMarkers $isolatedProfile
        if ($count -ne 0) { throw "expected 0 managed PATH markers after clear, found $count" }
    }
    finally {
        $PROFILE = $savedProfile
    }
}

function Smoke-PathNotice {
    $notice = 'does not resolve to'
    $savedPath = $env:Path
    try {
        $env:Path = "$env:PLANE_LOCAL_BIN_DIR$([System.IO.Path]::PathSeparator)$savedPath"
        $onPath = Capture-ConsoleError { & (Get-Manage) install --channel $channel --version $version --retain=false }
        if ($onPath -match $notice) {
            throw "install printed PATH notice even though $env:PLANE_LOCAL_BIN_DIR is on PATH"
        }
    }
    finally {
        $env:Path = $savedPath
    }
    $offPath = Capture-ConsoleError { & (Get-Manage) install --channel $channel --version $version --retain=false }
    if ($offPath -notmatch $notice) {
        throw "install did not print PATH notice when $env:PLANE_LOCAL_BIN_DIR is absent from PATH"
    }
}

function Smoke-Uninstall {
    $plane = Get-PlaneCmd
    & $plane skill uninstall
    if (Test-Path $skillPath) { throw "skill uninstall left $skillPath" }
    $list = (& $plane skill list | Out-String)
    if ($list -notmatch 'no managed skill installations') { throw "skill list still reports managed installations: $list" }
    & (Get-Manage) uninstall --version $version
    if (Test-Path (Join-Path $env:PLANE_LOCAL_BIN_DIR 'plane.cmd')) { throw 'version uninstall left plane.cmd' }
    if (Test-Path (Join-Path $env:PLANE_INSTALL_ROOT $version)) { throw "version uninstall left $(Join-Path $env:PLANE_INSTALL_ROOT $version)" }
}

function Smoke-Latest {
    foreach ($name in @('plane.cmd', 'plane.ps1', 'plane.exe')) {
        Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $env:PLANE_LOCAL_BIN_DIR $name)
    }
    $env:PLANE_INSTALL_ROOT = Join-Path $tmpdir 'latest-smoke'
    $env:PLANE_HOME = $env:PLANE_INSTALL_ROOT
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $env:PLANE_INSTALL_ROOT
    & (Get-Manage) install --channel $channel --install-root $env:PLANE_INSTALL_ROOT --retain=false
    $plane = Get-PlaneCmd
    if (!(Test-Path $plane)) { throw "latest install missing $plane" }
    & $plane --version
    & $plane help | Out-Null
    & (Get-Manage) uninstall --install-root $env:PLANE_INSTALL_ROOT
    if (Test-Path $env:PLANE_INSTALL_ROOT) { throw "full uninstall left $env:PLANE_INSTALL_ROOT" }
}

try {
    $python = Get-Python
    $mirrorRoot = Join-Path $tmpdir 'release-mirror'
    $port = Get-FreePort
    $mirrorUrl = "http://127.0.0.1:$port"
    & $python (Join-Path $root '.github/scripts/release/smoke/mirror.py') `
        --root $mirrorRoot `
        --channel $channel `
        --version $version `
        --platform 'plane-x86_64-pc-windows-msvc.zip' `
        --mirror-url $mirrorUrl
    $server = Start-Process -FilePath $python -ArgumentList @('-m', 'http.server', "$port", '--bind', '127.0.0.1') -WorkingDirectory $mirrorRoot -PassThru
    Wait-SmokeMirror "$mirrorUrl/$channel/versions/$version/metadata.json"

    $env:PLANE_INSTALL_ROOT = Join-Path (Join-Path $tmpdir 'home') '.local/share/plane'
    $env:PLANE_LOCAL_BIN_DIR = Join-Path $tmpdir 'bin'
    $env:PLANE_HOME = $env:PLANE_INSTALL_ROOT
    $env:PLANE_RELEASES_PUBLIC_URL = $mirrorUrl
    $skillPath = Join-Path $tmpdir 'agent/skills/plane-cli'
    New-Item -ItemType Directory -Force -Path $env:PLANE_INSTALL_ROOT, $env:PLANE_LOCAL_BIN_DIR | Out-Null
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $skillPath) | Out-Null

    Smoke-Install
    Smoke-CliBasics
    Smoke-ApiMeMock
    Smoke-SkillLifecycle
    Smoke-SkillContentBoundary
    Smoke-Upgrade
    Smoke-PathSetupClear
    Smoke-PathNotice
    Smoke-Uninstall

    if ($env:SMOKE_LATEST -eq '1') {
        Smoke-Latest
    }
}
finally {
    if ($server -and !$server.HasExited) {
        Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
        $server.WaitForExit()
    }
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $tmpdir
}
