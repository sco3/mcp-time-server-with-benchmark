#!/usr/bin/env -S bash

set -xueo pipefail

source ./token-from-file.sh

#echo $AUTH
#exit 
RUST_BACKTRACE=1

/usr/bin/time -v ./target/release/bench8 \
        --silent \
	--server $HOME/prj/mcp-stdio-wrapper/target/release/mcp_stdio_wrapper  \
	--url "http://localhost:3000/mcp/" \
	--auth "$AUTH" \
	--concurrency 150 \
	--log-level off \
	
exit
	--log-file bench-wrapper.log

#	--log-file bench-wrapper-inout.log  \
