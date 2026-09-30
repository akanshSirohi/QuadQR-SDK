@echo off
setlocal EnableExtensions
cd /d "%~dp0"

echo.
echo ========================================
echo   QuadQR Android AAR packaging
ECHO   Uses existing android-libs\*.so
echo ========================================
echo.

set "MISSING=0"
for %%A in (arm64-v8a armeabi-v7a x86_64) do (
    if not exist "android-libs\%%A\libquadqr.so" (
        echo [MISSING] android-libs\%%A\libquadqr.so
        set "MISSING=1"
    )
)
if "%MISSING%"=="1" (
    echo.
    echo [ERROR] Build the native libraries first with build-android.bat
    exit /b 1
)

call scripts\run-gradle.bat -p packages\quadqr-android clean assembleRelease
if errorlevel 1 exit /b 1

set "AAR=packages\quadqr-android\build\outputs\aar\quadqr-android-release.aar"
if not exist "%AAR%" (
    echo [ERROR] Gradle finished but the AAR was not found: %AAR%
    exit /b 1
)

if not exist dist mkdir dist
copy /y "%AAR%" "dist\quadqr-android-release.aar" >nul

echo.
echo [OK] AAR created:
echo   %CD%\dist\quadqr-android-release.aar
echo.
exit /b 0
