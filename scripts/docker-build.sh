#!/bin/bash
# Docker-based optimized build script

set -e

echo "🐳 Starting Docker-based Rust build..."

# Build the Docker image if needed
docker-compose build rust-builder

# Show sccache stats before
echo "📊 sccache stats (before):"
docker-compose run --rm rust-builder sccache --show-stats

# Perform the build
echo "🔨 Building with optimized Docker environment..."
time docker-compose run --rm rust-builder cargo build --release

# Show sccache stats after
echo "📊 sccache stats (after):"
docker-compose run --rm rust-builder sccache --show-stats

echo "✅ Docker build complete! Check target/release/ for artifacts."

# Optional: Copy artifacts to host
if [ "$1" = "--copy" ]; then
    echo "📁 Copying build artifacts..."
    docker-compose run --rm rust-builder cp target/release/libenhanced_football_game.so /workspace/bin/
    echo "✅ Artifacts copied to bin/"
fi