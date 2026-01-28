#!/usr/bin/env -S bash

set -ueo pipefail

source ./token-from-file.sh

time ./target/release/bench \
	bench.toml \
	-p \
	-i ${ITERS:=4} \
	-- \
	uv \
	--project $HOME/prj/mcp-context-forge run -m mcpgateway.wrapper --url "http://localhost:3000/mcp" \
	--auth "$AUTH" \
	--log-level off
