param([switch]$NoBundle,[ValidateSet('nsis')][string]$Bundle)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$projectDirectory = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Set-Location -LiteralPath $projectDirectory
$cargoDirectory = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path -LiteralPath (Join-Path $cargoDirectory 'cargo.exe')) {
    $env:PATH = "$cargoDirectory;$env:PATH"
}
try {
    foreach ($commandName in @('node.exe', 'npm.cmd', 'cargo.exe')) {
        if (-not (Get-Command $commandName -ErrorAction SilentlyContinue)) {
            throw "Build tool missing: $commandName. Regular launches only need RUN_WINDOWS.cmd."
        }
    }
    $executable = Join-Path $projectDirectory 'src-tauri\target\release\smart-photo-indexer.exe'
    $activeBuild = Get-CimInstance Win32_Process | Where-Object {
        $_.ExecutablePath -and [string]::Equals($_.ExecutablePath, $executable, [StringComparison]::OrdinalIgnoreCase)
    }
    if ($activeBuild) {
        throw 'Close this build normally before rebuilding. Other installations will not be interrupted.'
    }
    if (-not (Test-Path -LiteralPath 'node_modules\.bin\tauri.cmd') -or
        -not (Test-Path -LiteralPath 'node_modules\.bin\tsc.cmd')) {
        & npm.cmd ci --registry=https://registry.npmjs.org/
        if ($LASTEXITCODE -ne 0) { throw 'npm ci failed. The lockfile was preserved; build stopped.' }
    }
    & npm.cmd run verify:structure
    if ($LASTEXITCODE -ne 0) { throw 'Structure verification failed.' }
    if ($NoBundle) { & npm.cmd run tauri:build -- --no-bundle }
    elseif ($Bundle) { & npm.cmd run tauri:build -- --bundles $Bundle }
    else { & npm.cmd run tauri:build }
    if ($LASTEXITCODE -ne 0) { throw 'Desktop build failed.' }
    if (-not (Test-Path -LiteralPath $executable)) { throw 'The executable was not produced.' }
    Write-Host "Ready: $executable"
    exit 0
} catch {
    Write-Host $_.Exception.Message -ForegroundColor Red
    exit 1
}
