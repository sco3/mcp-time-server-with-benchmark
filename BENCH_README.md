# Bench - MCP Server Benchmarking Utility

A high-performance benchmarking utility for MCP (Model Context Protocol) servers that communicates via stdio.

## Features

- **Automated Benchmarking**: Run multiple benchmark steps with configurable payloads
- **Task Tracking**: Track individual request/response pairs with unique IDs
- **Timeout Handling**: Automatically detect and report timed-out requests (configurable TTL)
- **Statistics**: Calculate median, 99th percentile, standard deviation, min, and max response times
- **Multiple Iterations**: Run benchmarks multiple times for consistency
- **Flexible Configuration**: TOML-based configuration for easy customization
- **Logging**: Configurable log levels (trace, debug, info, warn, error)

## Installation

Build the bench utility:

```bash
cargo build --release --bin bench
```

The binary will be available at `./target/release/bench`

## Usage

Basic usage:

```bash
./target/release/bench <config-file> [OPTIONS] -- <command> [args...]
```

### Example

```bash
./target/release/bench \
    bench.toml \
    -i 4 \
    --log-level info \
    -- \
    fast-time-server
```

With a Python wrapper:

```bash
./target/release/bench \
    bench.toml \
    -p \
    -i 4 \
    --log-file bench-py-wrapper.bench.log \
    -- \
    uv \
    --project $HOME/prj/mcp-context-forge run -m mcpgateway.wrapper \
    --url "http://localhost:8844/mcp" \
    --auth "$AUTH" \
    --log-level debug
```

## Command Line Options

- `<config-file>`: Path to the TOML configuration file (required)
- `-p, --parallel`: Enable parallel execution mode (not yet implemented)
- `-i, --iterations <N>`: Number of iterations to run (default: 1)
- `--log-file <path>`: Path to log file (optional)
- `--log-level <level>`: Log level: trace, debug, info, warn, error (default: info)
- `-- <command> [args...]`: Command and arguments to execute (required)

## Configuration File Format

The configuration file uses TOML format:

```toml
# Optional: timeout for tasks in seconds (default: 60)
timeout_seconds = 60

[[steps]]
name = "Initialize"
bench = true  # Whether to measure timing for this step
payload = { jsonrpc = "2.0", id = 1, method = "initialize", params = { ... } }

[[steps]]
name = "Initialized Notification"
bench = false  # Don't measure timing for notifications
payload = { jsonrpc = "2.0", method = "notifications/initialized" }

[[steps]]
name = "System Time Call"
bench = true
tasks = 8000  # Number of times to send this payload (default: 1)
payload = { jsonrpc = "2.0", id = 3, method = "tools/call", params = { ... } }
```

### Configuration Fields

- `timeout_seconds` (optional): Timeout in seconds for task responses (default: 60)
- `steps`: Array of benchmark steps

#### Step Fields

- `name` (required): Name of the step for reporting
- `bench` (required): Whether to measure and report timing for this step
- `tasks` (optional): Number of times to send this payload (default: 1)
- `payload` (required): JSON-RPC payload to send

## How It Works

1. **Process Spawning**: Spawns the specified command with stdin/stdout pipes
2. **Task Tracking**: For each payload with an `id` field:
   - Assigns a unique ID (incrementing from 1)
   - Stores the payload and send time in a map
   - Sends the payload to the process stdin
3. **Response Handling**: 
   - Reads responses from process stdout
   - Matches responses to requests by ID
   - Calculates response time
   - Removes completed tasks from the map
4. **Timeout Detection**: 
   - Periodically checks for tasks exceeding the timeout
   - Marks timed-out tasks as failed
   - Removes them from tracking
5. **Statistics**: 
   - Calculates median, 99th percentile, std deviation
   - Reports successful and failed task counts

## Output

The utility provides detailed statistics for each benchmarked step:

```
=== Benchmark Results ===

Step: Initialize
  Total tasks:      1
  Successful:       1
  Failed:           0
  Median:           2.85ms
  99th percentile:  2.85ms
  Std deviation:    0μs
  Min:              2.85ms
  Max:              2.85ms

Step: System Time Call
  Total tasks:      8000
  Successful:       8000
  Failed:           0
  Median:           41.77ms
  99th percentile:  72.03ms
  Std deviation:    19.05ms
  Min:              8.70ms
  Max:              72.23ms
```

## Notes

- The utility automatically increments IDs for multiple tasks in a step
- Notifications (payloads without `id` field) are sent but not tracked
- Failed tasks are those that don't receive a response within the timeout period
- The process is terminated after all steps complete

## Examples

See the included configuration files:
- `bench.toml` - Full benchmark with 8000 tasks
- `bench-test.toml` - Test configuration with 10 tasks
- `bench-simple-test.toml` - Simple test with 5 tasks