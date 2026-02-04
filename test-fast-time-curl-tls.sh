#!/usr/bin/env -S bash

set -xueo pipefail

TOKEN_FILE="$HOME/.local/mcpgateway-bearer-token.txt"
if [ ! -f "$TOKEN_FILE" ]; then
        echo "Error: Token file not found at $TOKEN_FILE" >&2
        exit 1
fi

MCPGATEWAY_BEARER_TOKEN="Bearer $(tr -d '\r\n' <"$TOKEN_FILE")"

#MCPGATEWAY_BEARER_TOKEN="$(uv --project "${MCP_CONTEXT_FORGE_DIR}" run -m mcpgateway.utils.create_jwt_token --username admin@example.com --exp 10080 --secret my-test-key 2>/dev/null)"


URL="https://localhost:3000/mcp"

INIT='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"demo","version":"0.0.1"}}}'

NOTIFY='{"jsonrpc": "2.0","method": "notifications/initialized"}'
LIST='{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
TOOL_CALL='{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_system_time","arguments":{"timezone":"UTC"}}}'

HEADERS=(
        -k
	-H "Authorization: Bearer $MCPGATEWAY_BEARER_TOKEN"
	-H "Content-Type: application/json; charset=utf-8"
	-H "Accept: application/json, application/x-ndjson, text/event-stream"
)

curl -N "$URL" "${HEADERS[@]}" -d "$INIT"
printf "\n---\n"
curl  -N "$URL" "${HEADERS[@]}" -d "$NOTIFY"
printf "\n---\n"
curl  -N "$URL" "${HEADERS[@]}" -d "$LIST"
printf "\n---\n"
curl  -N "$URL" "${HEADERS[@]}" -d "$TOOL_CALL"
printf "\n---\n"
