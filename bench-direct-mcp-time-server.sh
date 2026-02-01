#!/usr/bin/env -S bash

set -xueo pipefail

source ./token-from-file.sh

time ./target/release/bench \
	--log-file bench-direct-time-server.log \
	--server ./target/release/mcp-time-server
