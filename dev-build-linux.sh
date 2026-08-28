#!/bin/bash
set -e

# Set UTF-8 encoding
export LC_ALL=C.UTF-8
export LANG=C.UTF-8

echo "Building Full Project for Linux (Dev Mode)..."
echo

# Check if .NET 8 SDK is installed
if ! command -v dotnet &> /dev/null; then
    echo "ERROR: .NET 8 SDK not found!"
    echo "Please install .NET 8 SDK from: https://dotnet.microsoft.com/download/dotnet/8.0"
else
    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        echo "ERROR: Rust not found!"
        echo "Please install Rust from: https://rustup.rs/"
    else
        echo "[1/4] Building CrawlCipher.Core (C#)..."
        cd CrawlCipher.Core
        dotnet restore
        dotnet build -c Release
        echo "Successfully built CrawlCipher.Core.dll"
        echo

        echo "[2/4] Publishing CrawlCipher.Core with NativeAOT..."
        dotnet publish -c Release -r linux-x64 -p:PublishAot=true
        echo "Successfully published CrawlCipher.Core"
        echo

        cd ..

        echo "[3/4] Building CrawlCipher.Tui (Rust)..."
        cd CrawlCipher.Tui
        cargo build --release
        echo "Successfully built crawlcipher terminal frontend"
        echo

        cd ..
        echo "[4/4] Copying files to output directory..."

        # Create output directory
        mkdir -p output

        # Copy executable
        rm -f output/crawlcipher
        cp CrawlCipher.Tui/target/release/crawlcipher output/crawlcipher

        # Copy C# library (rename to standard lib prefix)
        rm -f output/libCrawlCipher.Core.so
        cp CrawlCipher.Core/bin/Release/net8.0/linux-x64/publish/CrawlCipher.Core.so output/libCrawlCipher.Core.so

        # Also backup the library to core-binaries for public builds
        mkdir -p core-binaries
        cp CrawlCipher.Core/bin/Release/net8.0/linux-x64/publish/CrawlCipher.Core.so core-binaries/libCrawlCipher.Core.so

        # Make executable
        chmod +x output/crawlcipher

        # Create run script
        cat > output/run.sh << 'INNER_EOF'
#!/bin/bash
export LD_LIBRARY_PATH=.:$LD_LIBRARY_PATH
./crawlcipher "$@"
INNER_EOF

        chmod +x output/run.sh

        echo
        echo "Build completed successfully!"
        echo "Output files are in: output/"
        echo
        echo "Run the game with:"
        echo "  cd output && ./run.sh"
        echo
    fi
fi
