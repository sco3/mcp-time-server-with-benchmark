#!/usr/bin/env -S bash

set -xueo pipefail

source ./token-from-file.sh

#echo $AUTH
#exit 

time ./target/release/bench8 \
--silent false\
	--server  $HOME/prj/mcp-stdio-wrapper/target/release/mcp_stdio_wrapper  \
	--url "http://localhost:3000/mcp/" \
	--auth "$AUTH" \
	--log-level error \
	--log-file bench-wrapper-error.log 
	
	
#5	--log-file bench-wrapper-out.log \
