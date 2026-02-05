#!/usr/bin/env -S bash

set -ueo pipefail

source ./token-from-file.sh
export SSL_CERT_FILE=cert.pem
time ./target/release/bench8 \
	--log-file bench-py-wrapper.log \
	--server uv \
	--project $HOME/prj/mcp-context-forge run -m mcpgateway.wrapper \
	--url "https://localhost:3000/mcp" \
	--auth "$AUTH" \
	--log-level off
