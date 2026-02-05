#!/usr/bin/env -S bash

set -xueo pipefail

source ./token-from-file.sh

#echo $AUTH
#exit 

/usr/bin/time -v ./target/release/bench8 \
	--silent \
	--server $HOME/prj/mcp-stdio-wrapper/target/release/mcp_stdio_wrapper  \
	--url "https://localhost:3000/mcp/" \
	--auth "$AUTH" \
	--concurrency 32 \
	--tls-cert cert.pem \
	--http-pool-per-worker \
	--http-pool-size 1

exit 

	--log-file bench-wrapper-inout.log  \

	--log-level error \
	--log-file bench-wrapper.log
