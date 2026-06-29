$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))))
$version = if ($args.Length -gt 0) { $args[0] } else { throw 'missing release version' }
$channel = if ($args.Length -gt 1) { $args[1] } else { 'stable' }

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

    $env:PLANE_INSTALL_ROOT = Join-Path $tmpdir 'install'
    $env:PLANE_LOCAL_BIN_DIR = Join-Path $tmpdir 'bin'
    $env:PLANE_HOME = Join-Path $tmpdir 'plane-home'
    $env:PLANE_RELEASES_PUBLIC_URL = $mirrorUrl
    $skillPath = Join-Path $tmpdir 'agent/skills/plane-cli'
    New-Item -ItemType Directory -Force -Path $env:PLANE_INSTALL_ROOT, $env:PLANE_LOCAL_BIN_DIR | Out-Null
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $skillPath) | Out-Null
    & (Join-Path $root 'manage.ps1') install --channel $channel --version $version --retain=false
    & (Join-Path $env:PLANE_LOCAL_BIN_DIR 'plane.exe') --version
    & (Join-Path $env:PLANE_LOCAL_BIN_DIR 'plane.exe') help
    & (Join-Path $env:PLANE_LOCAL_BIN_DIR 'plane.exe') skill install --path $skillPath --channel $channel --version $version
    if (!(Test-Path (Join-Path $skillPath 'SKILL.md'))) {
        throw "skill install missing $(Join-Path $skillPath 'SKILL.md')"
    }
    & (Join-Path $root 'manage.ps1') upgrade --channel $channel --version $version --retain=false
    if (!(Test-Path (Join-Path $skillPath 'SKILL.md'))) {
        throw "skill upgrade missing $(Join-Path $skillPath 'SKILL.md')"
    }
    & (Join-Path $env:PLANE_LOCAL_BIN_DIR 'plane.exe') skill uninstall
    if (Test-Path $skillPath) {
        throw "skill uninstall left $skillPath"
    }
    & (Join-Path $root 'manage.ps1') uninstall --version $version
    if (Test-Path (Join-Path $env:PLANE_INSTALL_ROOT $version)) {
        throw "version uninstall left $(Join-Path $env:PLANE_INSTALL_ROOT $version)"
    }

    if ($env:SMOKE_LATEST -eq '1') {
        Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $env:PLANE_LOCAL_BIN_DIR 'plane.exe')
        $env:PLANE_INSTALL_ROOT = Join-Path $tmpdir 'latest-smoke'
        & (Join-Path $root 'manage.ps1') install --channel $channel --retain=false
        & (Join-Path $env:PLANE_LOCAL_BIN_DIR 'plane.exe') --version
        & (Join-Path $env:PLANE_LOCAL_BIN_DIR 'plane.exe') help
        & (Join-Path $root 'manage.ps1') uninstall --install-root $env:PLANE_INSTALL_ROOT
        if (Test-Path $env:PLANE_INSTALL_ROOT) {
            throw "full uninstall left $env:PLANE_INSTALL_ROOT"
        }
    }
}
finally {
    if ($server -and !$server.HasExited) {
        Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
        $server.WaitForExit()
    }
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $tmpdir
}
