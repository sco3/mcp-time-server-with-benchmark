#!/bin/bash

# Benchmark using rewrk: comparable to bench-hey.sh
# Usage: set AUTH="Bearer blah" and run this script

auth_header=${AUTH:+-H "Authorization: $AUTH"}

rewrk \
	-c 8 \
	-t 2 \
	-d 20s \
	-m POST \
	-b '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_system_time","arguments":{"timezone":"UTC"}}}' \
	-H "content-type: application/json" \
	$auth_header \
	-h "http://localhost:3000/mcp/"
