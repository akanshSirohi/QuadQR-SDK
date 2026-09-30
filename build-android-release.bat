@echo off
setlocal EnableExtensions
cd /d "%~dp0"

echo.
echo ========================================
echo   QuadQR complete Android release
ECHO   Rust .so libraries + Android AAR
echo ========================================
echo.

call build-android.bat
if errorlevel 1 exit /b 1

call build-aar.bat
if errorlevel 1 exit /b 1

echo.
echo ========================================
echo   QuadQR Android release complete
echo ========================================
echo.
echo Native libraries:
echo   %CD%\android-libs\
echo.
echo Android AAR:
echo   %CD%\dist\quadqr-android-release.aar
echo.
exit /b 0
