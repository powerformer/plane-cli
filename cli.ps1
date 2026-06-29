$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$command = if ($args.Length -gt 0) { $args[0] } else { 'help' }
$remaining = if ($args.Length -gt 1) { $args[1..($args.Length - 1)] } else { @() }

function Show-Usage {
    @'
plane repo operator

Usage:
  ./cli.ps1 land [options]
  ./cli.ps1 release --channel stable|beta [options]

Commands:
  land       Create or update the GitHub PR for the current branch.
  release    Trigger a release workflow.
'@ | Write-Output
}

switch ($command) {
    { $_ -in @('help', '-h', '--help') } {
        Show-Usage
        exit 0
    }
    { $_ -in @('land', 'release') } {}
    default {
        [Console]::Error.WriteLine("unknown command: $command")
        Show-Usage | ForEach-Object { [Console]::Error.WriteLine($_) }
        exit 2
    }
}

if (!(Get-Command uv -ErrorAction SilentlyContinue)) {
    throw 'missing dependency: uv'
}

$scripts = Join-Path $root 'scripts'
if ($env:PYTHONPATH) {
    $env:PYTHONPATH = "$scripts$([System.IO.Path]::PathSeparator)$env:PYTHONPATH"
}
else {
    $env:PYTHONPATH = $scripts
}

& uv run --project $scripts python -m "cli.$command" @remaining
exit $LASTEXITCODE
