#!/usr/bin/env pwsh
# Copyright 2018 the Deno authors. All rights reserved. MIT license.
# TODO(everyone): Keep this script simple and easily auditable.

$ErrorActionPreference = 'Stop'

if ($v) {
  $Version = "v${v}"
}
if ($args.Length -eq 1) {
  $Version = $args.Get(0)
}

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

$DvmUri = if (!$Version) {
  $Response = Invoke-WebRequest 'https://github.com/justjavac/dvm/releases' -UseBasicParsing
  if ($PSVersionTable.PSEdition -eq 'Core') {
    $Response.Links |
      Where-Object { $_.href -like "/justjavac/dvm/releases/download/*/dvm-${Target}.zip" } |
      ForEach-Object { 'https://github.com' + $_.href } |
      Select-Object -First 1
  } else {
    $HTMLFile = New-Object -Com HTMLFile
    if ($HTMLFile.IHTMLDocument2_write) {
      $HTMLFile.IHTMLDocument2_write($Response.Content)
    } else {
      $ResponseBytes = [Text.Encoding]::Unicode.GetBytes($Response.Content)
      $HTMLFile.write($ResponseBytes)
    }
    $HTMLFile.getElementsByTagName('a') |
      Where-Object { $_.href -like "about:/justjavac/dvm/releases/download/*/dvm-${Target}.zip" } |
      ForEach-Object { $_.href -replace 'about:', 'https://github.com' } |
      Select-Object -First 1
  }
} else {
  "https://github.com/justjavac/dvm/releases/download/${Version}/dvm-${Target}.zip"
}

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