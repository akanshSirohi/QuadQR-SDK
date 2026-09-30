$ErrorActionPreference = 'Stop'

function Get-CompatibleJavaHome([string]$candidate) {
    if (-not $candidate) { return $null }
    $java = Join-Path $candidate 'bin/java.exe'
    $release = Join-Path $candidate 'release'
    if (-not (Test-Path -LiteralPath $java) -or -not (Test-Path -LiteralPath $release)) {
        return $null
    }
    $match = [regex]::Match((Get-Content -LiteralPath $release -Raw), 'JAVA_VERSION="(\d+)')
    if (-not $match.Success) { return $null }
    $major = [int]$match.Groups[1].Value
    if ($major -ge 17 -and $major -le 23) {
        return (Resolve-Path -LiteralPath $candidate).Path
    }
    [Console]::Error.WriteLine("[WARNING] Skipping Java $major at $candidate; Gradle 8.11.1 needs Java 17-23.")
    return $null
}

$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$candidates = [System.Collections.Generic.List[string]]::new()
$seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)

if ($env:QUADQR_JAVA_HOME) {
    $selected = Get-CompatibleJavaHome $env:QUADQR_JAVA_HOME
    if ($selected) { Write-Output $selected; exit 0 }
    [Console]::Error.WriteLine('[ERROR] QUADQR_JAVA_HOME must point to a JDK 17-23 installation.')
    exit 1
}

if ($env:JAVA_HOME) { $candidates.Add($env:JAVA_HOME) }
$localJdks = Join-Path $repoRoot '.tools/jdk-21'
if (Test-Path -LiteralPath $localJdks) {
    Get-ChildItem -LiteralPath $localJdks -Directory | ForEach-Object { $candidates.Add($_.FullName) }
}
$candidates.Add('C:\Program Files\Android\Android Studio\jbr')
Get-Command java.exe -All -ErrorAction SilentlyContinue | ForEach-Object {
    $candidates.Add((Split-Path -Parent (Split-Path -Parent $_.Source)))
}

foreach ($candidate in $candidates) {
    if (-not $seen.Add($candidate)) { continue }
    $selected = Get-CompatibleJavaHome $candidate
    if ($selected) { Write-Output $selected; exit 0 }
}

[Console]::Error.WriteLine('[ERROR] No compatible JDK found. Install JDK 17 or 21 and set QUADQR_JAVA_HOME.')
exit 1
