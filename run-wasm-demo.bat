@echo off
setlocal
cd /d "%~dp0packages\\quadqr-js"

call npm run setup:wasm || goto :fail
call npm run check:source || goto :fail
call npm run build:wasm || goto :fail
call npm run verify:wasm || goto :fail
call npm run check || goto :fail
call npm run serve || goto :fail
exit /b 0

:fail
echo.
echo QuadQR WASM demo setup stopped because a previous command failed.
exit /b 1
