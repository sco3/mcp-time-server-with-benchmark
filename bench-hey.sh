hey -n 500000 -c 8 \
  -m POST \
  -H "Authorization: $AUTH" \
  -T "application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_system_time","arguments":{"timezone":"UTC"}}}' \
  "http://localhost:3000/mcp/"