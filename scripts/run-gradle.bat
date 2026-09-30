@echo off
setlocal EnableExtensions

set "REPO_ROOT=%~dp0.."
for %%I in ("%REPO_ROOT%") do set "REPO_ROOT=%%~fI"
set "GRADLE_VERSION=8.11.1"
set "TOOLS_DIR=%REPO_ROOT%\.tools"
set "GRADLE_HOME=%TOOLS_DIR%\gradle-%GRADLE_VERSION%"
set "GRADLE_EXE=%GRADLE_HOME%\bin\gradle.bat"

rem Select a JDK supported by this Gradle version before starting it.
call "%REPO_ROOT%\scripts\setup-android-java.bat"
if errorlevel 1 exit /b 1

if not defined ANDROID_HOME if exist "%LOCALAPPDATA%\Android\Sdk" set "ANDROID_HOME=%LOCALAPPDATA%\Android\Sdk"
if not defined ANDROID_SDK_ROOT if defined ANDROID_HOME set "ANDROID_SDK_ROOT=%ANDROID_HOME%"

if not exist "%GRADLE_EXE%" (
    echo [INFO] Gradle %GRADLE_VERSION% is not cached. Downloading it once...
    if not exist "%TOOLS_DIR%" mkdir "%TOOLS_DIR%"
    set "GRADLE_ZIP=%TOOLS_DIR%\gradle-%GRADLE_VERSION%-bin.zip"

    powershell -NoProfile -ExecutionPolicy Bypass -Command ^
      "$ErrorActionPreference='Stop'; $ProgressPreference='SilentlyContinue'; Invoke-WebRequest -UseBasicParsing 'https://services.gradle.org/distributions/gradle-%GRADLE_VERSION%-bin.zip' -OutFile '%TOOLS_DIR%\gradle-%GRADLE_VERSION%-bin.zip'"
    if errorlevel 1 (
        echo [ERROR] Could not download Gradle %GRADLE_VERSION%.
        exit /b 1
    )

    powershell -NoProfile -ExecutionPolicy Bypass -Command ^
      "$ErrorActionPreference='Stop'; Expand-Archive -LiteralPath '%TOOLS_DIR%\gradle-%GRADLE_VERSION%-bin.zip' -DestinationPath '%TOOLS_DIR%' -Force"
    if errorlevel 1 (
        echo [ERROR] Could not extract Gradle.
        exit /b 1
    )

    del /q "%TOOLS_DIR%\gradle-%GRADLE_VERSION%-bin.zip" >nul 2>nul
)

if not exist "%GRADLE_EXE%" (
    echo [ERROR] Gradle executable was not found after setup.
    exit /b 1
)

echo [INFO] Running Gradle %GRADLE_VERSION% with a compatible JDK.
call "%GRADLE_EXE%" %*
exit /b %ERRORLEVEL%
