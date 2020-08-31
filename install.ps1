#!/usr/bin/env pwsh
# Copyright 2018 the Deno authors. All rights reserved. MIT license.
# TODO(everyone): Keep this script simple and easily auditable.

$ErrorActionPreference = 'Stop'

$DenoInstall = $env:DENO_INSTALL
$DenoBinDir = if ($DenoInstall) {
  "$DvmDir\bin"
} else {
  "$Home\.deno\bin"
}

$DvmDir = $env:DVM_DIR
$BinDir = if ($DvmDir) {
  "$DvmDir\bin"
} else {
  "$Home\.dvm\bin"
}

$DvmZip = "$BinDir\dvm.zip"
$DvmExe = "$BinDir\dvm.exe"
$Target = 'x86_64-pc-windows-msvc'

# GitHub requires TLS 1.2
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$DvmUri = "https://cdn.jsdelivr.net/gh/justjavac/dvm@latest/dvm-${Target}.zip"

if (!(Test-Path $BinDir)) {
  New-Item $BinDir -ItemType Directory | Out-Null
}

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

if (!(";$Path;".ToLower() -like "*;$DenoInstall;*".ToLower())) {
  [Environment]::SetEnvironmentVariable('Path', "$Path;$DenoInstall", $User)
  $Env:Path += ";$DenoInstall"
}
if (!(";$Path;".ToLower() -like "*;$BinDir;*".ToLower())) {
  [Environment]::SetEnvironmentVariable('Path', "$Path;$BinDir", $User)
  $Env:Path += ";$BinDir"
}

Write-Output "Dvm was installed successfully to $DvmExe"
Write-Output "Run 'dvm --help' to get started"