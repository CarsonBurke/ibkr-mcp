# ibkr-mcp

Read-only MCP server for Interactive Brokers via [rust-ibapi](https://github.com/wboayue/rust-ibapi). Exposes tools for news, historical bars, contract lookup, and account data over HTTP. No trading, no mutations.

Requires TWS or IB Gateway running locally.

## Install

```sh
cargo install --path .
```

This puts `ibkr-mcp` in `~/.cargo/bin/`. Works on Linux, macOS, and Windows.

## Run

```sh
ibkr-mcp                    # listen on http://127.0.0.1:3099/mcp
ibkr-mcp --port 4000        # custom port
ibkr-mcp --ibkr-addr 127.0.0.1:7497  # connect to TWS instead of Gateway
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
claude mcp add -s user --transport http ibkr http://127.0.0.1:3099/mcp
```

Tools appear as `mcp__ibkr__news_headlines`, `mcp__ibkr__historical_bars`, etc.

Manage with `claude mcp list`, `claude mcp remove ibkr`.

## Adding to Codex CLI

```sh
codex mcp add --transport http ibkr http://127.0.0.1:3099/mcp
```

## Connection

Default: `127.0.0.1:4002` (IB Gateway paper trading). TWS uses port 7497 (paper) or 7496 (live). Override with `--ibkr-addr`.
