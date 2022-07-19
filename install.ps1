#!/usr/bin/env pwsh
# Copyright 2018 the Deno authors. All rights reserved. MIT license.
# Copyright 2022 justjavac. All rights reserved. MIT license.
# TODO(everyone): Keep this script simple and easily auditable.

$ErrorActionPreference = 'Stop'

$DvmDir = $env:DVM_DIR
$BinDir = if ($DvmDir) {
  "$DvmDir\bin"
} else {
  "$Home\.dvm\bin"
}

$DvmZip = "$BinDir\dvm.zip"
$DvmExe = "$BinDir\dvm.exe"

# GitHub requires TLS 1.2
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$DvmUri = "https://cdn.jsdelivr.net/gh/justjavac/dvm_releases@main/dvm-x86_64-pc-windows-msvc.zip"

if (!(Test-Path $BinDir)) {
  New-Item $BinDir -ItemType Directory | Out-Null
}

# Ignore SSL certificate errors
# https://stackoverflow.com/questions/11696944/powershell-v3-invoke-webrequest-https-error
add-type @"
    using System.Net;
    using System.Security.Cryptography.X509Certificates;
    public class TrustAllCertsPolicy : ICertificatePolicy {
        public bool CheckValidationResult(
            ServicePoint srvPoint, X509Certificate certificate,
            WebRequest request, int certificateProblem) {
            return true;
        }
    }
"@
[System.Net.ServicePointManager]::CertificatePolicy = New-Object TrustAllCertsPolicy

Invoke-WebRequest $DvmUri -OutFile $DvmZip -UseBasicParsing

if (Get-Command Expand-Archive -ErrorAction SilentlyContinue) {
  Expand-Archive $DvmZip -Destination $BinDir -Force
} else {
  if (Test-Path $DvmExe) {
    Remove-Item $DvmExe
  }
  Add-Type -AssemblyName System.IO.Compression.FileSystem
  [IO.Compression.ZipFile]::ExtractToDirectory($DvmZip, $BinDir)
}

Remove-Item $DvmZip

$User = [EnvironmentVariableTarget]::User
$Path = [Environment]::GetEnvironmentVariable('Path', $User)

if (!(";$Path;".ToLower() -like "*;$BinDir;*".ToLower())) {
  [Environment]::SetEnvironmentVariable('Path', "$Path;$BinDir", $User)
  $Env:Path += ";$BinDir"
}

Write-Output "Dvm was installed successfully to $DvmExe"
Invoke-Expression -Command "dvm doctor"
Write-Output "Run 'dvm --help' to get started"
