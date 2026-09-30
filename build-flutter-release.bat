@echo off
setlocal EnableExtensions
cd /d "%~dp0"

echo.
echo ========================================
echo   QuadQR Flutter release package
echo ========================================
echo.

call build-flutter.bat
if errorlevel 1 exit /b 1

where flutter >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Flutter SDK is required to validate a Flutter release package.
    exit /b 1
)

if not exist dist mkdir dist
if exist dist\quadqr-flutter rmdir /s /q dist\quadqr-flutter
mkdir dist\quadqr-flutter

xcopy /e /i /y packages\quadqr-flutter dist\quadqr-flutter >nul
if errorlevel 1 exit /b 1

if exist dist\quadqr-flutter\.dart_tool rmdir /s /q dist\quadqr-flutter\.dart_tool
if exist dist\quadqr-flutter\build rmdir /s /q dist\quadqr-flutter\build
if exist dist\quadqr-flutter\android\.gradle rmdir /s /q dist\quadqr-flutter\android\.gradle
if exist dist\quadqr-flutter\example\.dart_tool rmdir /s /q dist\quadqr-flutter\example\.dart_tool
if exist dist\quadqr-flutter\example\build rmdir /s /q dist\quadqr-flutter\example\build
if exist dist\quadqr-flutter\pubspec.lock del /q dist\quadqr-flutter\pubspec.lock
if exist dist\quadqr-flutter\example\pubspec.lock del /q dist\quadqr-flutter\example\pubspec.lock

if exist dist\quadqr-flutter.zip del /q dist\quadqr-flutter.zip
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "$ErrorActionPreference='Stop'; Compress-Archive -Path 'dist\quadqr-flutter\*' -DestinationPath 'dist\quadqr-flutter.zip' -Force"
if errorlevel 1 exit /b 1

echo.
echo [OK] Flutter package archive created:
echo   %CD%\dist\quadqr-flutter.zip
exit /b 0
