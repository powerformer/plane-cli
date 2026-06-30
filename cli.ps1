$ErrorActionPreference = 'Stop'

$commands = @('release', 'init')

function Show-Usage {
    [Console]::Error.WriteLine('usage: ./cli.ps1 :<command> [args...]')
    [Console]::Error.WriteLine("commands: $($commands -join ' ')")
}

if ($args.Length -lt 1) {
    Show-Usage
    exit 1
}

$first = [string]$args[0]
if (-not $first.StartsWith(':')) {
    Show-Usage
    exit 1
}

$name = $first.Substring(1)
if ($commands -notcontains $name) {
    [Console]::Error.WriteLine("unknown command: :$name")
    Show-Usage
    exit 1
}

if (!(Get-Command uv -ErrorAction SilentlyContinue)) {
    throw 'missing dependency: uv'
}

$module = $name -replace '-', '_'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$remaining = if ($args.Length -gt 1) { $args[1..($args.Length - 1)] } else { @() }
$scripts = Join-Path $root 'scripts'
if ($env:PYTHONPATH) {
    $env:PYTHONPATH = "$scripts$([System.IO.Path]::PathSeparator)$env:PYTHONPATH"
}
else {
    $env:PYTHONPATH = $scripts
}

& uv run --project $scripts python -m "cli.$module" @remaining
exit $LASTEXITCODE
