#!/usr/bin/env -S bash

set -ueo pipefail

source ./token-from-file.sh

time ./target/release/bench8 \
	--log-file bench-py-wrapper.log \
	--server uv \
	--project $HOME/prj/mcp-context-forge run -m mcpgateway.wrapper \
	--url "http://localhost:3000/mcp" \
	--auth "$AUTH" \
	--log-level off
