#!/usr/bin/env -S bash

set -ueo pipefail

source ./token-from-file.sh

time ./target/release/bench \
	bench.toml \
	-p \
	-i ${ITERS:=4} \
	--log-file bench-py-wrapper.bench.log \
	-- \
	uv \
	--project $HOME/prj/mcp-context-forge run -m mcpgateway.wrapper --url "http://localhost:8844/mcp" \
	--auth "$AUTH" \
	--log-level debug
