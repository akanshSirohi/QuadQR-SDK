@echo off
setlocal EnableExtensions
cd /d "%~dp0"
call scripts\setup-android-java.bat
if errorlevel 1 exit /b 1
echo.
echo Java executable:
echo   %JAVA_HOME%\bin\java.exe
echo.
"%JAVA_HOME%\bin\java.exe" -version
exit /b %ERRORLEVEL%
