#!/usr/bin/env -S bash

set -xueo pipefail

source ./token-from-file.sh

#echo $AUTH
#exit 

/usr/bin/time -v ./target/release/bench \
	--log-file bench-wrapper-inout.log  \
	--server $HOME/prj/mcp-stdio-wrapper/target/release/mcp_stdio_wrapper  \
	--url "https://localhost:3000/mcp/" \
	--auth "$AUTH" \
	--log-level error \
	--tls-cert cert.pem  \
	--log-file bench-wrapper.log
