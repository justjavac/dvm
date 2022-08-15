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

curl.exe -Lo $DvmZip $DvmUri

tar.exe xf $DvmZip -C $BinDir

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
