@echo off
setlocal EnableExtensions DisableDelayedExpansion
chcp 65001 >nul
title Smart Photo Indexer

set "APP_EXE=%~dp0src-tauri\target\release\smart-photo-indexer.exe"

if not exist "%APP_EXE%" (
  echo Готовая сборка приложения не найдена.
  echo Нужно подготовить сборку, затем повторить запуск.
  echo.
  echo Ожидаемый файл: "%APP_EXE%"
  echo.
  pause
  exit /b 1
)

start "" /D "%~dp0" "%APP_EXE%" %*
if errorlevel 1 (
  echo Не удалось запустить Smart Photo Indexer.
  echo Файл: "%APP_EXE%"
  echo.
  pause
  exit /b 1
)

exit /b 0
