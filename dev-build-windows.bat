@echo off
chcp 65001 >nul
echo Building Full Project for Windows (Dev Mode)...
echo.

REM Check if .NET 8 SDK is installed
dotnet --version >nul 2>&1
if errorlevel 1 (
    echo ERROR: .NET 8 SDK not found!
    echo Please install .NET 8 SDK from: https://dotnet.microsoft.com/download/dotnet/8.0
    pause
    goto :EOF
)

REM Check if Rust is installed
cargo --version >nul 2>&1
if errorlevel 1 (
    echo ERROR: Rust not found!
    echo Please install Rust from: https://rustup.rs/
    pause
    goto :EOF
)

echo [1/4] Building CrawlCipher.Core (C#)...
cd CrawlCipher.Core
dotnet restore
dotnet build -c Release
if errorlevel 1 (
    echo ERROR: Failed to build Core!
    pause
    goto :EOF
)
echo Successfully built CrawlCipher.Core.dll
echo.

echo [2/4] Publishing CrawlCipher.Core with NativeAOT...
dotnet publish -c Release -r win-x64 -p:PublishAot=true
if errorlevel 1 (
    echo ERROR: Failed to publish Core!
    pause
    goto :EOF
)
echo Successfully published CrawlCipher.Core
echo.

cd ..

echo [3/4] Building CrawlCipher.Tui (Rust)...
cd CrawlCipher.Tui
cargo build --release
if errorlevel 1 (
    echo ERROR: Failed to build Terminal Frontend!
    pause
    goto :EOF
)
echo Successfully built crawlcipher.exe
echo.

echo [4/4] Copying files to output directory...
cd ..

REM Create output directory
if not exist "output\" mkdir output

REM Copy executable
copy "CrawlCipher.Tui\target\release\crawlcipher.exe" "output\crawlcipher.exe"

REM Copy C# library
copy "CrawlCipher.Core\bin\Release\net8.0\win-x64\publish\CrawlCipher.Core.dll" "output\CrawlCipher.Core.dll"

REM Also backup the library to core-binaries for public builds
if not exist "core-binaries\" mkdir core-binaries
copy "CrawlCipher.Core\bin\Release\net8.0\win-x64\publish\CrawlCipher.Core.dll" "core-binaries\CrawlCipher.Core.dll"

echo.
echo Build completed successfully!
echo Output files are in: output\
echo.
echo Run the game with: cd output ^&^& crawlcipher.exe
pause
