# ibkr-mcp

MCP server for Interactive Brokers via [rust-ibapi](https://github.com/wboayue/rust-ibapi). Exposes read-only tools for news, historical bars, contract lookup, and account data over stdio. No trading, no mutations.

Requires TWS or IB Gateway running locally.

## Build

```sh
cargo build --release
```

## Tools

| Tool | Description |
|-|-|
| `news_providers` | List available news sources and codes |
| `news_headlines` | Historical headlines for a ticker (symbol, providers, limit, since) |
| `news_article` | Read full article body by provider + article ID |
| `contract_details` | Detailed contract info for a ticker |
| `contract_search` | Search symbols by name or partial ticker |
| `historical_bars` | OHLCV bars (symbol, duration, bar_size, show) |
| `account_summary` | Balances, margin, buying power |
| `positions` | All open positions |

## Adding to Claude Code

```sh
claude mcp add -s user ibkr -- target/release/ibkr-mcp
```

Or for project scope only:

```sh
claude mcp add ibkr -- target/release/ibkr-mcp
```

Tools appear as `mcp__ibkr__news_headlines`, `mcp__ibkr__historical_bars`, etc.

Manage with `claude mcp list`, `claude mcp remove ibkr`.

## Adding to Codex CLI

```sh
codex mcp add ibkr -- /path/to/ibkr-mcp/target/release/ibkr-mcp
```

## Connection

Default: `127.0.0.1:4002` (IB Gateway paper trading), client ID 99. To change, edit `src/main.rs` — the address is hardcoded to avoid runtime config complexity. TWS uses port 7497 (paper) or 7496 (live).
