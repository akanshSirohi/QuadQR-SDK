@echo off
setlocal EnableExtensions
cd /d "%~dp0"

rem Use the same Android Studio JBR as the AAR build when available.
call scripts\setup-android-java.bat
if errorlevel 1 exit /b 1

echo.
echo ========================================
echo   QuadQR Flutter Android package setup
echo   Rust .so + Flutter FFI plugin
echo ========================================
echo.

call build-android.bat
if errorlevel 1 exit /b 1

call scripts\sync-flutter-libs.bat
if errorlevel 1 exit /b 1

where flutter >nul 2>nul
if errorlevel 1 (
    echo.
    echo [WARNING] Flutter SDK was not found in PATH.
    echo The native libraries were built and synchronized successfully.
    echo Install Flutter or add flutter\bin to PATH, then run this file again
    echo to execute flutter pub get and flutter analyze.
    exit /b 0
)

pushd packages\quadqr-flutter
flutter pub get
if errorlevel 1 (
    popd
    exit /b 1
)

flutter analyze
if errorlevel 1 (
    popd
    exit /b 1
)
popd

echo.
echo [OK] QuadQR Flutter package is ready:
echo   %CD%\packages\quadqr-flutter
exit /b 0
