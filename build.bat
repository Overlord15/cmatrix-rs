@echo off
REM ------------------------------------------------------------
REM  Build script for cmatrix (Rust) on Windows
REM  Usage: double-click or run  build.bat  from this folder
REM  Output: dist\cmatrix.exe
REM ------------------------------------------------------------
setlocal
cd /d "%~dp0"

where cargo >nul 2>nul
if errorlevel 1 (
    echo [!] Rust / cargo was not found.
    echo     Install it from https://rustup.rs  ^(choose the default MSVC option^),
    echo     then open a NEW terminal and run build.bat again.
    exit /b 1
)

echo [*] Building release binary...
cargo build --release
if errorlevel 1 (
    echo [!] Build failed.
    exit /b 1
)

if not exist dist mkdir dist
copy /Y "target\release\cmatrix.exe" "dist\cmatrix.exe" >nul
if errorlevel 1 (
    echo [!] Could not copy the executable.
    exit /b 1
)

echo.
echo [+] Done:  %cd%\dist\cmatrix.exe
echo     Run it with:  dist\cmatrix.exe
endlocal
