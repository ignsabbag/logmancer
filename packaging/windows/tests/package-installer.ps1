$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot '../package-installer.ps1'
$temporary = Join-Path ([System.IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
$installers = Join-Path $temporary 'installer source with spaces'
$output = Join-Path $temporary 'output with spaces'
New-Item -ItemType Directory -Path $installers -Force | Out-Null
try {
    $original = Join-Path $installers 'Logmancer_0.5.0-beta.4_x64-setup.exe'
    [System.IO.File]::WriteAllText($original, 'installer fixture')
    foreach ($version in @('0.5.0-beta.4', '0.5.0')) {
        & $script -Version $version -InstallerDirectory $installers -OutputDirectory $output
        $baseName = "logmancer-$version-windows-x86_64-setup"
        $zip = [System.IO.Compression.ZipFile]::OpenRead((Join-Path $output "$baseName.zip"))
        try {
            if ($zip.Entries.Count -ne 1 -or $zip.Entries[0].FullName -ne "$baseName.exe") {
                throw 'Setup ZIP must contain exactly one normalized EXE at its root.'
            }
            $reader = [System.IO.StreamReader]::new($zip.Entries[0].Open())
            try {
                if ($reader.ReadToEnd() -ne 'installer fixture') {
                    throw 'Installer contents changed.'
                }
            } finally {
                $reader.Dispose()
            }
        } finally {
            $zip.Dispose()
        }
    }
    if (!(Test-Path -LiteralPath $original) -or @(Get-ChildItem $output -Filter '*.exe').Count -ne 0) {
        throw 'Packaging must preserve the source and publish no loose EXE.'
    }
    foreach ($count in @(0, 2)) {
        $invalid = Join-Path $temporary "invalid-$count"
        New-Item -ItemType Directory -Path $invalid | Out-Null
        for ($i = 0; $i -lt $count; $i++) {
            Copy-Item -LiteralPath $original -Destination (Join-Path $invalid "$i.exe")
        }
        $rejected = $false
        try {
            & $script -Version '0.5.0' -InstallerDirectory $invalid -OutputDirectory $output
        } catch {
            $rejected = $true
        }
        if (!$rejected) { throw "Expected rejection of $count installers." }
    }
    Write-Output 'Windows installer packaging tests passed.'
} finally {
    Remove-Item -LiteralPath $temporary -Recurse -Force
}
