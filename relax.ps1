# Build the pinned Relax runner on first use, then forward its command line.
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$revision = '5dc818e7d969906ccc90d756844f595b9491e846'
$patch = Join-Path $PSScriptRoot 'patches/relax.patch'
$patchHash = (Get-FileHash -LiteralPath $patch -Algorithm SHA256).Hash.Substring(0, 12)
$cache = Join-Path $PSScriptRoot "target/relax-tool/$($revision.Substring(0, 8))-$patchHash"
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
if (-not ((Test-Path -LiteralPath $runner -PathType Leaf) -and (Test-Path -LiteralPath $ready -PathType Leaf))) {
    [IO.Directory]::CreateDirectory($cache) | Out-Null
    $lock = [Threading.Mutex]::new($false, "TeamyCapabilityLabRelaxSetup-$patchHash")
    try { [void]$lock.WaitOne() } catch [Threading.AbandonedMutexException] { }
    try {
        if (-not (Test-Path -LiteralPath (Join-Path $source 'Cargo.toml'))) {
            Invoke-SetupStep git @('init', '--quiet', $source)
            Invoke-SetupStep git @('-C', $source, 'fetch', '--quiet', '--depth=1', 'https://github.com/makepad/relax.git', $revision)
            Invoke-SetupStep git @('-C', $source, 'checkout', '--quiet', '--detach', $revision)
        }
        # A build can be retried after the patch has already been applied.
        $alreadyPatched = & {
            # Windows PowerShell reports the expected failed check on its error stream.
            $ErrorActionPreference = 'Continue'
            & git -C $source apply --reverse --check -- $patch 2>$null
            $LASTEXITCODE -eq 0
        }
        if (-not $alreadyPatched) {
            Invoke-SetupStep git @('-C', $source, 'apply', '--', $patch)
        }
        Invoke-SetupStep cargo @('build', '--release', '--offline', '--manifest-path', (Join-Path $source 'Cargo.toml'), '--target-dir', $build, '-p', 'relax-runner', '--bin', 'relax')
        [IO.File]::WriteAllText($ready, $revision)
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
