#!/bin/bash

# Benchmark script for MCP time-server get_system_time tool
# Uses hey utility to perform load testing

# Configuration
URL="http://localhost:3000/mcp"
REQUESTS=10000
CONCURRENCY=50
DURATION=10  # seconds

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}MCP Time Server - get_system_time Benchmark${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check if server is running
echo -e "${YELLOW}Checking if server is running...${NC}"
if ! curl -s -o /dev/null -w "%{http_code}" "$URL" > /dev/null 2>&1; then
    echo -e "${YELLOW}Warning: Server might not be running at $URL${NC}"
    echo -e "${YELLOW}Please start the server with: cargo run --release${NC}"
    echo ""
fi

# Create JSON-RPC request payload (from bench.toml)
cat > /tmp/get-system-time-request.json << 'EOF'
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "get_system_time",
    "arguments": {
      "timezone": "UTC"
    }
  }
}
EOF

echo -e "${GREEN}Test Configuration:${NC}"
echo "  URL: $URL"
echo "  Total Requests: $REQUESTS"
echo "  Concurrency: $CONCURRENCY"
echo "  Duration: ${DURATION}s"
echo "  Tool: get_system_time"
echo "  Timezone: UTC"
echo ""

echo -e "${BLUE}Running benchmark...${NC}"
echo ""

# Run hey benchmark
hey -n $REQUESTS \
    -c $CONCURRENCY \
    -z ${DURATION}s \
    -m POST \
    -H "Content-Type: application/json" \
    -D /tmp/get-system-time-request.json \
    "$URL"

# Cleanup
rm -f /tmp/get-system-time-request.json

echo ""
echo -e "${GREEN}Benchmark complete!${NC}"

# Made with Bob