#!/usr/bin/env bash
set -euo pipefail

BUILD_MODE=""
TRUNK_FLAGS=""
CARGO_FLAGS=""

# Parse arguments
if [[ "${1:-}" == "--release" ]]; then
    BUILD_MODE="release"
    TRUNK_FLAGS="--cargo-profile wasm-release"
    CARGO_FLAGS="--release"
    echo "🔧 Building in RELEASE mode"
else
    echo "🔧 Building in DEBUG mode (pass --release for optimized build)"
fi

# Check for Trunk
if ! command -v trunk &> /dev/null; then
    echo "❌ trunk is not installed. Please run: cargo install trunk"
    exit 1
fi

# Build frontend with trunk
echo "📦 Building Yew frontend with Trunk..."
pushd frontend > /dev/null
trunk build $TRUNK_FLAGS
popd > /dev/null

# Build backend with cargo
echo "🛠️ Building Actix backend with Cargo..."
cargo build $CARGO_FLAGS

echo "✅ Build ($BUILD_MODE) completed successfully!"
