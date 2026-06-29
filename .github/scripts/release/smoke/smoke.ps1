$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))))
$version = if ($args.Length -gt 0) { $args[0] } else { throw 'missing release version' }
$channel = if ($args.Length -gt 1) { $args[1] } else { 'stable' }

$tmpdir = Join-Path ([System.IO.Path]::GetTempPath()) ("plane-smoke-" + [System.Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmpdir | Out-Null

try {
    $env:PLANE_INSTALL_ROOT = Join-Path $tmpdir 'install'
    $env:PLANE_LOCAL_BIN_DIR = Join-Path $tmpdir 'bin'
    New-Item -ItemType Directory -Force -Path $env:PLANE_INSTALL_ROOT, $env:PLANE_LOCAL_BIN_DIR | Out-Null
    & (Join-Path $root 'manage.ps1') install --channel $channel --version $version --retain=false
    & (Join-Path $env:PLANE_LOCAL_BIN_DIR 'plane.exe') --version
    & (Join-Path $env:PLANE_LOCAL_BIN_DIR 'plane.exe') help
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
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $tmpdir
}
