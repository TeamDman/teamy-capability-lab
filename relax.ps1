# Build the pinned Relax runner on first use, then forward its command line.
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$manifest = Join-Path $PSScriptRoot 'Cargo.toml'
$metadata = cargo metadata --no-deps --offline --format-version 1 --manifest-path $manifest | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw "Could not read $manifest with cargo metadata" }
$package = $metadata.packages | Where-Object { $_.name -eq 'teamy-capability-lab' }
$pin = $package.metadata.relax
if (-not $pin.git -or $pin.rev -cnotmatch '^[0-9a-f]{40}$') {
    throw 'Set git and a full commit SHA in [package.metadata.relax] in Cargo.toml'
}
$revision = $pin.rev
$cache = Join-Path $PSScriptRoot "target/relax-tool/$revision"
$source = Join-Path $cache 'src'
$build = Join-Path $cache 'build'
$binary = if ($env:OS -eq 'Windows_NT') { 'relax.exe' } else { 'relax' }
$runner = Join-Path $build "release/$binary"

function Invoke-SetupStep($program, [string[]]$arguments) {
    & $program @arguments | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "$program $($arguments -join ' ') failed with exit code $LASTEXITCODE"
    }
}

$ready = Join-Path $cache 'ready'
function Test-RunnerReady {
    (Test-Path -LiteralPath $runner -PathType Leaf) -and (Test-Path -LiteralPath $ready -PathType Leaf)
}

if (-not (Test-RunnerReady)) {
    [IO.Directory]::CreateDirectory($cache) | Out-Null
    $lock = [Threading.Mutex]::new($false, "TeamyCapabilityLabRelaxSetup-$revision")
    try { [void]$lock.WaitOne() } catch [Threading.AbandonedMutexException] { }
    try {
        if (-not (Test-RunnerReady)) {
            if (-not (Test-Path -LiteralPath (Join-Path $source 'Cargo.toml'))) {
                Invoke-SetupStep git @('init', '--quiet', $source)
                Invoke-SetupStep git @('-C', $source, 'fetch', '--quiet', '--depth=1', '--no-tags', $pin.git, $revision)
            }
            Invoke-SetupStep git @('-C', $source, '-c', 'core.longpaths=true', 'checkout', '--quiet', '--detach', $revision)
            Push-Location -LiteralPath $source
            try {
                Invoke-SetupStep cargo @('build', '--release', '--target-dir', $build, '-p', 'relax-runner', '--bin', 'relax')
            } finally {
                Pop-Location
            }
            [IO.File]::WriteAllText($ready, $revision)
        }
    } finally {
        $lock.ReleaseMutex()
        $lock.Dispose()
    }
}

# Relax builds this package's native dependencies for the Windows GNU ABI.
$previousPath = $env:PATH
$gnuBin = Join-Path $PSScriptRoot 'target/llvm-mingw/bin'
if (($env:OS -eq 'Windows_NT') -and (Test-Path -LiteralPath (Join-Path $gnuBin 'x86_64-w64-mingw32-gcc.exe'))) {
    $env:PATH = $gnuBin + [IO.Path]::PathSeparator + $previousPath
}
Push-Location -LiteralPath $PSScriptRoot
try {
    & $runner @args
    $status = $LASTEXITCODE
} finally {
    Pop-Location
    $env:PATH = $previousPath
}
exit $status
