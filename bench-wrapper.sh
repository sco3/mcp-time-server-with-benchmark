#!/usr/bin/env -S bash

set -xueo pipefail

source ./token-from-file.sh

time ./target/release/bench \
	--log-file bench-wrapper-out.log \
	--server mcp_stdio_wrapper \
	--url "http://localhost:3000/mcp/" \
	--auth "$AUTH" \
	--log-level off 
