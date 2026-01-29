#!/usr/bin/env -S bash

set -ueo pipefail

rm -f out.log

EXE=(fast-time-server)

INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"}}}'
NOTIFY='{"jsonrpc":"2.0","id":2, "method":"notifications/initialized"}'
LIST='{"jsonrpc":"2.0","id":3,"method":"tools/list"}'

CALL='{"jsonrpc": "2.0","id": 4,"method": "tools/call","params": {"name": "get_system_time","arguments": {}}}'
TEMPLATE='{"jsonrpc": "2.0","id": REPLACE_ID,"method": "tools/call","params": {"name": "get_system_time","arguments": {}}}'
(
	echo "$INIT"
	sleep 0.2
	echo "$NOTIFY"
	sleep 0.2
	echo "$LIST"
	sleep 0.2
	for i in {4..100}; do
		echo "${TEMPLATE/REPLACE_ID/$i}"
	done
) | "${EXE[@]}"
