#!/usr/bin/env -S bash

set -xueo pipefail

source ./token-from-file.sh

time ./target/release/bench \
	bench.toml \
	--persistent \
	--iterations ${ITERS:=4} \
	--log-file bench-wrapper-out.log \
	-- \
	mcp_stdio_wrapper \
	--url "http://localhost:8844/mcp/" \
	--auth "$AUTH" \
	--log-level debug \
	--log-file bench-wrapper.log
