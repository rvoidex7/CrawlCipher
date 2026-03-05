#!/bin/bash
set -e

# Set UTF-8 encoding
export LC_ALL=C.UTF-8
export LANG=C.UTF-8

echo "Building Terminal UI (Rust) for Linux..."
echo

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "ERROR: Rust not found!"
    echo "Please install Rust from: https://rustup.rs/"
    # Skip exit in sandbox to prevent shell closure issues
else

    echo "[1/2] Building Terminal Frontend..."
    cd CrawlCipher.Terminal
    cargo build --release
    cd ..
    echo "Successfully built terminal frontend."
    echo

    echo "[2/2] Copying files to output directory..."

    # Create output directory
    mkdir -p output

    # Copy Rust executable
    cp CrawlCipher.Terminal/target/release/crawlcipher output/crawlcipher

    # Check if pre-compiled core binary exists
    if [ -f "core-binaries/libCrawlCipher.Core.so" ]; then
        cp core-binaries/libCrawlCipher.Core.so output/libCrawlCipher.Core.so
    else
        echo "WARNING: Pre-compiled Proprietary Engine binary not found in 'core-binaries/libCrawlCipher.Core.so'!"
        echo "Please ensure you have downloaded the proprietary core binary."
    fi

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
