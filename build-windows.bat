@echo off
chcp 65001 >nul
echo Building Terminal UI (Rust) for Windows...
echo.

REM Check if Rust is installed
cargo --version >nul 2>&1
if errorlevel 1 (
    echo ERROR: Rust not found!
    echo Please install Rust from: https://rustup.rs/
    pause
    goto :EOF
)

echo [1/2] Building Terminal Frontend...
cd CrawlCipher.Terminal
cargo build --release
if errorlevel 1 (
    echo ERROR: Failed to build Terminal Frontend!
    pause
    goto :EOF
)
echo Successfully built crawlcipher.exe
echo.

echo [2/2] Copying files to output directory...
cd ..

REM Create output directory
if not exist "output\" mkdir output

REM Copy Rust executable
copy "CrawlCipher.Terminal\target\release\crawlcipher.exe" "output\crawlcipher.exe"

REM Check and copy pre-compiled Proprietary library
if exist "core-binaries\CrawlCipher.Core.dll" (
    copy "core-binaries\CrawlCipher.Core.dll" "output\CrawlCipher.Core.dll"
) else (
    echo WARNING: Pre-compiled Proprietary Engine binary not found in 'core-binaries\CrawlCipher.Core.dll'!
    echo Please ensure you have downloaded the proprietary core binary.
)

echo.
echo Build completed successfully!
echo Output files are in: output\
echo.
echo Run the game with: cd output ^&^& crawlcipher.exe
pause
