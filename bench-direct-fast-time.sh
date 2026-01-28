#!/usr/bin/env -S bash

set -xueo pipefail

source ./token-from-file.sh

time ./target/release/bench \
	bench.toml \
	--persistent \
	--iterations ${ITERS:=4} \
	--log-file bench-direct.log \
	-- \
	fast-time-server
