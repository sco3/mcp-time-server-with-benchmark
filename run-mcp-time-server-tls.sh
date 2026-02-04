#!/usr/bin/env -S bash


set -xueo pipefail

tmux new-session -d -s mcp-time-server 'cargo run --release --bin mcp-time-server -- --tls-cert cert.pem --tls-key key.pem "$@"'