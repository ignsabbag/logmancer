param(
    [Parameter(Mandatory)][string]$Version,
    [Parameter(Mandatory)][string]$InstallerDirectory,
    [Parameter(Mandatory)][string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$installers = @(Get-ChildItem -LiteralPath $InstallerDirectory -Filter '*.exe' -File)
if ($installers.Count -ne 1) {
    throw "Expected exactly one NSIS installer, found $($installers.Count)."
}

New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$baseName = "logmancer-$Version-windows-x86_64-setup"
$staging = Join-Path ([System.IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $staging | Out-Null
try {
    $executable = Join-Path $staging "$baseName.exe"
    Copy-Item -LiteralPath $installers[0].FullName -Destination $executable
    Compress-Archive -LiteralPath $executable -DestinationPath (Join-Path $OutputDirectory "$baseName.zip") -Force
} finally {
    Remove-Item -LiteralPath $staging -Recurse -Force
}
