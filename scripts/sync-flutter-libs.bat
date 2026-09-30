@echo off
setlocal EnableExtensions
set "REPO_ROOT=%~dp0.."
for %%I in ("%REPO_ROOT%") do set "REPO_ROOT=%%~fI"
cd /d "%REPO_ROOT%"

set "DEST=packages\quadqr-flutter\android\src\main\jniLibs"
set "MISSING=0"

for %%A in (arm64-v8a armeabi-v7a x86_64) do (
    if not exist "android-libs\%%A\libquadqr.so" (
        echo [MISSING] android-libs\%%A\libquadqr.so
        set "MISSING=1"
    )
)

if "%MISSING%"=="1" (
    echo [ERROR] Native libraries are missing. Run build-android.bat first.
    exit /b 1
)

for %%A in (arm64-v8a armeabi-v7a x86_64) do (
    if not exist "%DEST%\%%A" mkdir "%DEST%\%%A"
    copy /y "android-libs\%%A\libquadqr.so" "%DEST%\%%A\libquadqr.so" >nul
    if errorlevel 1 exit /b 1
    echo [OK] Flutter %%A\libquadqr.so
)

exit /b 0
