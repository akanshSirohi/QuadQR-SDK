@echo off
setlocal EnableExtensions
cd /d "%~dp0"

echo.
echo ========================================
echo   QuadQR Android native build
ECHO   arm64-v8a + armeabi-v7a + x86_64
echo ========================================
echo.

where cargo >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Rust/Cargo was not found.
    echo Install Rust from https://rustup.rs and run this file again.
    exit /b 1
)

where rustup >nul 2>nul
if errorlevel 1 (
    echo [ERROR] rustup was not found.
    exit /b 1
)

cargo ndk --version >nul 2>nul
if errorlevel 1 (
    echo [ERROR] cargo-ndk is not installed.
    echo Run: cargo install cargo-ndk
    exit /b 1
)

if not defined ANDROID_HOME if exist "%LOCALAPPDATA%\Android\Sdk" set "ANDROID_HOME=%LOCALAPPDATA%\Android\Sdk"
if not defined ANDROID_SDK_ROOT if defined ANDROID_HOME set "ANDROID_SDK_ROOT=%ANDROID_HOME%"

set "CARGO_NDK_PLATFORM=21"
set "OUT_DIR=%CD%\android-libs"

echo [1/3] Installing Rust Android targets if needed...
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
if errorlevel 1 exit /b 1

echo.
echo [2/3] Building QuadQR native libraries...
if exist "%OUT_DIR%" rmdir /s /q "%OUT_DIR%"

cargo ndk ^
    -t arm64-v8a ^
    -t armeabi-v7a ^
    -t x86_64 ^
    -o "%OUT_DIR%" ^
    build --release -p quadqr-native
if errorlevel 1 (
    echo.
    echo [ERROR] Native Android build failed.
    echo If the NDK is not detected, set ANDROID_NDK_HOME to your installed NDK folder.
    exit /b 1
)

echo.
echo [3/3] Verifying output...
set "FAILED=0"
for %%A in (arm64-v8a armeabi-v7a x86_64) do (
    if exist "%OUT_DIR%\%%A\libquadqr.so" (
        echo [OK] %%A\libquadqr.so
    ) else (
        echo [MISSING] %%A\libquadqr.so
        set "FAILED=1"
    )
)

if "%FAILED%"=="1" (
    echo [ERROR] One or more Android libraries were not generated.
    exit /b 1
)

echo.
echo Native build complete:
echo   %OUT_DIR%
echo.
exit /b 0
