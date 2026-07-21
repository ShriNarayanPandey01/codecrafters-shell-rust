@echo off
setlocal

set "COMMAND=%~1"
set "PREFIX=%~2"
set "PREVIOUS=%~3"

if /I "%PREVIOUS%"=="democtl" goto root
if /I "%PREVIOUS%"=="--env" goto env
if /I "%PREVIOUS%"=="config" goto config
goto root

:root
echo start
echo stop
echo status
echo config
echo --help
echo --env
goto end

:env
echo dev
echo staging
echo prod
goto end

:config
echo show
echo reset
echo path

:end
endlocal
