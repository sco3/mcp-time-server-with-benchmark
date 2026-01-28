#!/usr/bin/env -S bash

set -ueo pipefail

#export RUST_LOG="reqwest=trace,hyper=trace"

TOKEN_FILE="$HOME/.local/mcpgateway-bearer-token.txt"
if [ ! -f "$TOKEN_FILE" ]; then
	echo "Error: Token file not found at $TOKEN_FILE" >&2
	exit 1
fi

AUTH="Bearer $(tr -d '\r\n' <"$TOKEN_FILE")"

rm -f out.log

if [[ "${P:=X}" == "P" ]]; then
	EXE=(
		uv
		--project ~/prj/mcp-context-forge
		run
		-m
		mcpgateway.wrapper
		--url "http://localhost:3000/mcp"
		--auth "$AUTH"
		--log-level off
	)
else
	EXE=(
		mcp_stdio_wrapper
		--url "http://localhost:3000/mcp"
		--auth "$AUTH"
		--log-level off
		#--log-file out.log
	)
fi

INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0"}}}'
NOTIFY='{"jsonrpc":"2.0","method":"notifications/initialized"}'
LIST='{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
CALL='{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_system_time","arguments":{"timezone":"UTC"}}}'

SLEEP=${SLEEP:=0.01}

time (
	echo "$INIT"
	sleep ${SLEEP}
	echo "$NOTIFY"
	sleep ${SLEEP}
	echo "$LIST"
	sleep ${SLEEP}
	echo "$CALL"
	sleep ${SLEEP}
) | "${EXE[@]}"
