# MCP Time Server Benchmark

This document describes how to benchmark the MCP time server using the `hey` HTTP load testing tool.

## Prerequisites

- `hey` utility installed (https://github.com/rakyll/hey)
- MCP time server running on `localhost:3000`

## Running the Benchmark

### 1. Start the Server (in release mode for best performance)

```bash
cargo run --release
```

### 2. Run the Benchmark Script

```bash
./benchmark-tools-list.sh
```

## Benchmark Configuration

The script tests the `tools/list` endpoint with the following default settings:

- **URL**: `http://localhost:3000/mcp`
- **Total Requests**: 10,000
- **Concurrency**: 50 concurrent workers
- **Duration**: 30 seconds
- **Method**: POST with JSON-RPC 2.0 payload

## Customizing the Benchmark

You can modify the script variables at the top of `benchmark-tools-list.sh`:

```bash
REQUESTS=10000      # Total number of requests
CONCURRENCY=50      # Number of concurrent workers
DURATION=30         # Test duration in seconds
```

## Manual Benchmark Commands

### Basic benchmark (10,000 requests, 50 concurrent)
```bash
hey -n 10000 -c 50 -m POST \
    -H "Content-Type: application/json" \
    -D <(echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}') \
    http://localhost:3000/mcp
```

### Time-based benchmark (30 seconds)
```bash
hey -z 30s -c 50 -m POST \
    -H "Content-Type: application/json" \
    -D <(echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}') \
    http://localhost:3000/mcp
```

### High concurrency test (200 workers)
```bash
hey -n 50000 -c 200 -m POST \
    -H "Content-Type: application/json" \
    -D <(echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}') \
    http://localhost:3000/mcp
```

## Understanding the Results

The `hey` tool provides detailed statistics including:

- **Requests/sec**: Throughput of the server
- **Response time distribution**: Percentiles (50th, 90th, 95th, 99th)
- **Latency**: Average, slowest, and fastest response times
- **Status code distribution**: Success/error rates

## Example Output

```
Summary:
  Total:        30.0234 secs
  Slowest:      0.0523 secs
  Fastest:      0.0012 secs
  Average:      0.0145 secs
  Requests/sec: 3331.2345
  
Response time histogram:
  0.001 [1]     |
  0.006 [2345]  |■■■■■■■■■■■■■■■■■■■■
  0.011 [5678]  |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
  ...

Latency distribution:
  10% in 0.0089 secs
  25% in 0.0112 secs
  50% in 0.0134 secs
  75% in 0.0167 secs
  90% in 0.0201 secs
  95% in 0.0234 secs
  99% in 0.0312 secs
```

## Tips for Accurate Benchmarking

1. **Use release builds**: Always benchmark with `cargo run --release`
2. **Warm up**: Run a small test first to warm up the server
3. **System resources**: Close unnecessary applications
4. **Network**: Test on localhost to eliminate network latency
5. **Multiple runs**: Run the benchmark multiple times and average results