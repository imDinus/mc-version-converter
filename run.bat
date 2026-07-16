@echo off
chcp 65001 > nul
cd /d "%~dp0"

if not exist "target\release\mcconvert.exe" (
    echo mcconvert.exe not found. Build it first: cargo build --release
    pause
    exit /b 1
)

set "PATH=%CD%\target\release;%PATH%"

echo =================================================
echo   MC Version Converter
echo =================================================
echo.
echo Examples:
echo   mcconvert batch ^<version^>       convert every world in "input_worlds"
echo   mcconvert info "world path"      show a world's version
echo   mcconvert versions               list supported versions
echo.

cmd /k
