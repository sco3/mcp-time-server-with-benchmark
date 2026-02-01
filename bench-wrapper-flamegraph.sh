#!/usr/bin/env -S bash

set -xueo pipefail

source ./token-from-file.sh

# Install cargo-flamegraph if not present
if ! command -v flamegraph &> /dev/null; then
    echo "Installing cargo-flamegraph..."
    cargo install flamegraph
fi

# Build the bench tool if needed
cargo build --release

# Run the benchmark under flamegraph profiling
# The bench tool will start mcp_stdio_wrapper as a subprocess
# and flamegraph will profile both the bench tool and the wrapper
flamegraph -o mcp-stdio-wrapper-flamegraph.svg -- \
    ./target/release/bench \
    --log-file bench-wrapper-flamegraph-out.log \
    --server mcp_stdio_wrapper \
    --url "http://localhost:3000/mcp/" \
    --auth "$AUTH" \
    --log-level off

echo "Flamegraph generated: mcp-stdio-wrapper-flamegraph.svg"
