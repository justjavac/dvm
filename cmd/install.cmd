REM @echo off
set /P APP_PATH = %1
setx /M DVM_HOME "%APP_PATH%\.dvm"
REM setx /M DVM_SYMLINK "%APP_PATH%\deno"
REM setx /M PATH "%PATH%;%DVM_HOME%;%DVM_SYMLINK%"
REM @echo on
