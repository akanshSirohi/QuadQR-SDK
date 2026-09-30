@echo off
setlocal EnableExtensions

rem Gradle 8.11.1 needs a JDK from 17 through 23.
rem Android Studio's bundled JBR may be newer, so validate it before use.
set "SELECTED_JAVA_HOME="
for /f "usebackq delims=" %%J in (`powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0select-android-java.ps1"`) do set "SELECTED_JAVA_HOME=%%J"
if not defined SELECTED_JAVA_HOME exit /b 1

echo [INFO] Android build Java:
echo   JAVA_HOME=%SELECTED_JAVA_HOME%
"%SELECTED_JAVA_HOME%\bin\java.exe" -version
if errorlevel 1 exit /b 1

endlocal & set "JAVA_HOME=%SELECTED_JAVA_HOME%" & set "PATH=%SELECTED_JAVA_HOME%\bin;%PATH%"
exit /b 0
