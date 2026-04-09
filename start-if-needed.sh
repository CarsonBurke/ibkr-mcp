#!/bin/sh
# Start ibkr-mcp HTTP server if not already running.
# Used as a PreToolUse hook by Claude Code.

PORT=${IBKR_MCP_PORT:-3099}
BIN="$(command -v ibkr-mcp || echo "$(dirname "$0")/target/release/ibkr-mcp")"

if curl -sf "http://127.0.0.1:$PORT/mcp" -o /dev/null 2>/dev/null; then
    exit 0
fi

if ! [ -x "$BIN" ]; then
    echo "ibkr-mcp binary not found at $BIN — run: cargo build --release" >&2
    exit 1
fi

nohup "$BIN" --port "$PORT" > /tmp/ibkr-mcp.log 2>&1 &

# Wait up to 5s for server to be ready
for i in 1 2 3 4 5; do
    sleep 1
    if curl -sf "http://127.0.0.1:$PORT/mcp" -o /dev/null 2>/dev/null; then
        exit 0
    fi
done

echo "ibkr-mcp failed to start — check /tmp/ibkr-mcp.log" >&2
exit 1
