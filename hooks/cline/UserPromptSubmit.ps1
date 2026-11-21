# Inject pending skill-capture context before the next model request.
$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$env:OMEGA_SKILL_CAPTURE_ROOT = $root
$python = Get-Command python -ErrorAction SilentlyContinue
if (-not $python) { $python = Get-Command python3 -ErrorAction SilentlyContinue }
if (-not $python) {
    @{ cancel = $false; errorMessage = "Python 3 is required for skill-capture hooks" } | ConvertTo-Json -Compress
    exit 0
}
[Console]::In.ReadToEnd() | & $python.Source -B (Join-Path $root "hooks\cline\skill_capture_hook.py") prompt
