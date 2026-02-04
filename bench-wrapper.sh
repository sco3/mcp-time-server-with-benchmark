#!/usr/bin/env -S bash

set -xueo pipefail

source ./token-from-file.sh

#echo $AUTH
#exit 

/usr/bin/time -v ./target/release/bench8 \
	--silent \
	--server $HOME/prj/mcp-stdio-wrapper/target/release/mcp_stdio_wrapper  \
	--url "http://localhost:3000/mcp/" \
	--auth "$AUTH" \
	--log-level off \
#	--log-file bench-wrapper-error.log 
#	--log-file bench-wrapper-out.log \
