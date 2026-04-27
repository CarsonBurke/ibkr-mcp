# ibkr-mcp

Local based, read-only MCP server for Interactive Brokers via [rust-ibapi](https://github.com/wboayue/rust-ibapi). Exposes tools for news, historical bars, contract lookup, and account data over HTTP.

Requires TWS or IB Gateway running locally.

## Install

```sh
cargo install --path .
```

This puts `ibkr-mcp` in `~/.cargo/bin/` or equivalent for Linux, macOS, and Windows.

## Run

```sh
ibkr-mcp                    # listen on http://127.0.0.1:3099/mcp, auto-discover TWS/Gateway
ibkr-mcp --port 4000        # custom MCP port
ibkr-mcp --ibkr-addr 127.0.0.1:7497  # pin to TWS paper instead of auto-discovering
```

If `--ibkr-addr` is omitted, the server probes `127.0.0.1` on `4001` (Gateway live), `4002` (Gateway paper), `7497` (TWS paper), `7496` (TWS live) and uses the first that accepts a connection.

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

## Usage and Skills

See [example-skills/analyze-stock/SKILL.md](example-skills/analyze-stock/SKILL.md) instructing an agent on how to use `ibkr-mcp` for market analysis.

## Connection

Auto-discovers a TWS or IB Gateway listener on `127.0.0.1`, in this order:

| Port | App | Mode |
|-|-|-|
| 4001 | IB Gateway | Live |
| 4002 | IB Gateway | Paper |
| 7497 | TWS | Paper |
| 7496 | TWS | Live |

Override with `--ibkr-addr host:port` to pin a specific endpoint.
