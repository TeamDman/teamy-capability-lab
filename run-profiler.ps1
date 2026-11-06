param(
	[Parameter(ValueFromRemainingArguments = $true)]
	[string[]]$QueryArgs
)

if (-not $QueryArgs -or $QueryArgs.Count -eq 0) {
	$QueryArgs = @("plan")
}

$profiler = Get-Command teamy-profiler -ErrorAction SilentlyContinue
if (-not $profiler) {
	throw "teamy-profiler not found in PATH"
}

& $profiler.Source run cargo `
	--project $PSScriptRoot `
	--bin teamy-capability-lab `
	--profile release `
	--feature tracy `
	-- @QueryArgs
exit $LASTEXITCODE
