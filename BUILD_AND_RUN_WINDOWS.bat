@echo off
setlocal EnableExtensions DisableDelayedExpansion
chcp 65001 >nul
rem Kept as a launch alias for existing shortcuts. Build separately with BUILD_RELEASE_WINDOWS.bat.
call "%~dp0RUN_WINDOWS.cmd" %*
exit /b %errorlevel%
