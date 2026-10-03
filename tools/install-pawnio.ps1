$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$directory = Join-Path $projectRoot 'target\prerequisites'
New-Item -ItemType Directory -Force -Path $directory | Out-Null
$installer = Join-Path $directory 'PawnIO_setup-2.2.0.exe'
Invoke-WebRequest 'https://github.com/namazso/PawnIO.Setup/releases/download/2.2.0/PawnIO_setup.exe' -OutFile $installer
$signature = Get-AuthenticodeSignature -LiteralPath $installer
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'CN=namazso.eu') {
    throw 'Official installer Authenticode signature validation failed.'
}
$process = Start-Process -FilePath $installer -ArgumentList '-install', '-silent' -Verb RunAs -WindowStyle Hidden -Wait -PassThru
if ($process.ExitCode -ne 0) { throw "PawnIO installer failed: $($process.ExitCode)" }
Write-Output 'Official PawnIO installed. Start TaskbarMetrics.exe --sensors to enable CPU temperature.'
