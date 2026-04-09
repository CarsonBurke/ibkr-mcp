use std::sync::Arc;

use ibapi::accounts::types::AccountGroup;
use ibapi::accounts::{AccountSummaryResult, AccountSummaryTags, PositionUpdate};
use ibapi::client::blocking::Client;
use ibapi::contracts::Contract;
use ibapi::market_data::historical::{BarSize, Duration, WhatToShow};
use ibapi::market_data::TradingHours;
use rmcp::handler::server::{router::tool::ToolRouter, wrapper::Parameters};
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct IbkrServer {
    client: Arc<Client>,
    tool_router: ToolRouter<Self>,
}

impl IbkrServer {
    pub fn new(client: Arc<Client>) -> Self {
        Self {
            client,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_handler]
impl ServerHandler for IbkrServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(concat!(
                "Read-only Interactive Brokers market data. ",
                "Tools: news (providers, headlines, articles), contracts (details, search), ",
                "historical OHLCV bars, account summary, and positions.\n\n",
                "If this server is not reachable, start it with:\n",
                "  ibkr-mcp\n\n",
                "Requires TWS or IB Gateway on 127.0.0.1:4002.",
            ))
    }
}

// --- Request types ---

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SymbolRequest {
    #[schemars(description = "Ticker symbol, e.g. AAPL, TE, SPY")]
    pub symbol: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchRequest {
    #[schemars(description = "Search pattern — partial symbol or company name")]
    pub pattern: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HeadlinesRequest {
    #[schemars(description = "Ticker symbol")]
    pub symbol: String,
    #[schemars(description = "Comma-separated provider codes (e.g. BRFG,DJNL). Empty string or omit for all.")]
    pub providers: Option<String>,
    #[schemars(description = "Max results, 1-300")]
    pub limit: Option<u8>,
    #[schemars(description = "Start time as RFC3339 (e.g. 2026-01-01T00:00:00Z). Default: 30 days ago.")]
    pub since: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ArticleRequest {
    #[schemars(description = "Provider code from headlines output, e.g. BRFG")]
    pub provider: String,
    #[schemars(description = "Article ID from headlines output")]
    pub article_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BarsRequest {
    #[schemars(description = "Ticker symbol")]
    pub symbol: String,
    #[schemars(description = "Duration: 1D, 5D, 1W, 1M, 3M, 6M, 1Y. Default: 1M")]
    pub duration: Option<String>,
    #[schemars(description = "Bar size: 1min, 5min, 15min, 30min, 1h, 1d, 1w, 1m. Default: 1d")]
    pub bar_size: Option<String>,
    #[schemars(description = "Data type: trades, midpoint, bid, ask. Default: trades")]
    pub show: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AccountSummaryRequest {
    #[schemars(description = "Account group. Default: All")]
    pub group: Option<String>,
}

// --- Tool implementations ---

#[tool_router]
impl IbkrServer {
    #[tool(description = "List available IBKR news providers and their codes")]
    async fn news_providers(&self) -> String {
        let client = self.client.clone();
        tokio::task::spawn_blocking(move || match client.news_providers() {
            Ok(providers) => {
                let mut lines = vec!["Code | Name".into(), "--- | ---".into()];
                for p in &providers {
                    lines.push(format!("{} | {}", p.code, p.name));
                }
                lines.join("\n")
            }
            Err(e) => format!("Error: {e}"),
        })
        .await
        .unwrap_or_else(|e| format!("Task error: {e}"))
    }

    #[tool(description = "Get news headlines for a ticker symbol")]
    async fn news_headlines(&self, Parameters(req): Parameters<HeadlinesRequest>) -> String {
        let client = self.client.clone();
        tokio::task::spawn_blocking(move || {
            let contract = Contract::stock(&req.symbol).build();
            let details = match client.contract_details(&contract) {
                Ok(d) => d,
                Err(e) => return format!("Error resolving symbol: {e}"),
            };
            let contract_id = match details.first() {
                Some(d) => d.contract.contract_id,
                None => return format!("Unknown symbol: {}", req.symbol),
            };

            let provider_codes: Vec<&str> = match &req.providers {
                Some(p) if !p.is_empty() => p.split(',').collect(),
                _ => vec![],
            };

            let end = OffsetDateTime::now_utc();
            let start = match &req.since {
                Some(s) => match OffsetDateTime::parse(s, &Rfc3339) {
                    Ok(t) => t,
                    Err(e) => return format!("Invalid since time: {e}"),
                },
                None => end - time::Duration::days(30),
            };

            let limit = req.limit.unwrap_or(30);

            match client.historical_news(contract_id, &provider_codes, start, end, limit) {
                Ok(sub) => {
                    let mut lines =
                        vec!["Time | Provider | Headline | Article ID".into(), "--- | --- | --- | ---".into()];
                    for article in sub.iter() {
                        lines.push(format!(
                            "{} | {} | {} | {}",
                            article.time.format(&Rfc3339).unwrap_or_default(),
                            article.provider_code,
                            article.headline,
                            article.article_id,
                        ));
                    }
                    lines.join("\n")
                }
                Err(e) => format!("Error: {e}"),
            }
        })
        .await
        .unwrap_or_else(|e| format!("Task error: {e}"))
    }

    #[tool(description = "Read a full news article by provider code and article ID")]
    async fn news_article(&self, Parameters(req): Parameters<ArticleRequest>) -> String {
        let client = self.client.clone();
        tokio::task::spawn_blocking(move || match client.news_article(&req.provider, &req.article_id) {
            Ok(body) => body.article_text.clone(),
            Err(e) => format!("Error: {e}"),
        })
        .await
        .unwrap_or_else(|e| format!("Task error: {e}"))
    }

    #[tool(description = "Look up detailed contract info for a ticker")]
    async fn contract_details(&self, Parameters(req): Parameters<SymbolRequest>) -> String {
        let client = self.client.clone();
        tokio::task::spawn_blocking(move || {
            let contract = Contract::stock(&req.symbol).build();
            match client.contract_details(&contract) {
                Ok(all) => {
                    let mut out = Vec::new();
                    for d in &all {
                        let c = &d.contract;
                        out.push(format!(
                            "Symbol: {}\nName: {}\nType: {}\nExchange: {}\nCurrency: {}\nContract ID: {}\nIndustry: {}\nCategory: {}\nSubcategory: {}\nMarket: {}\nMin Tick: {}",
                            c.symbol, d.long_name, c.security_type, c.exchange, c.currency,
                            c.contract_id, d.industry, d.category, d.subcategory, d.market_name, d.min_tick,
                        ));
                    }
                    out.join("\n\n")
                }
                Err(e) => format!("Error: {e}"),
            }
        })
        .await
        .unwrap_or_else(|e| format!("Task error: {e}"))
    }

    #[tool(description = "Search for matching ticker symbols by name or partial symbol")]
    async fn contract_search(&self, Parameters(req): Parameters<SearchRequest>) -> String {
        let client = self.client.clone();
        tokio::task::spawn_blocking(move || match client.matching_symbols(&req.pattern) {
            Ok(results) => {
                let mut lines = vec!["Symbol | Type | Exchange | Currency | ID".into(), "--- | --- | --- | --- | ---".into()];
                for desc in results {
                    let c = &desc.contract;
                    lines.push(format!("{} | {} | {} | {} | {}", c.symbol, c.security_type, c.exchange, c.currency, c.contract_id));
                }
                lines.join("\n")
            }
            Err(e) => format!("Error: {e}"),
        })
        .await
        .unwrap_or_else(|e| format!("Task error: {e}"))
    }

    #[tool(description = "Get historical OHLCV price bars for a ticker")]
    async fn historical_bars(&self, Parameters(req): Parameters<BarsRequest>) -> String {
        let client = self.client.clone();
        tokio::task::spawn_blocking(move || {
            let contract = Contract::stock(&req.symbol).build();
            let duration = parse_duration(req.duration.as_deref().unwrap_or("1M"));
            let bar_size = parse_bar_size(req.bar_size.as_deref().unwrap_or("1d"));
            let what = parse_what_to_show(req.show.as_deref().unwrap_or("trades"));

            match client.historical_data(&contract, None, duration, bar_size, what, TradingHours::Regular) {
                Ok(data) => {
                    let mut lines = vec!["Date | Open | High | Low | Close | Volume".into(), "--- | --- | --- | --- | --- | ---".into()];
                    for bar in &data.bars {
                        lines.push(format!(
                            "{} | {:.2} | {:.2} | {:.2} | {:.2} | {:.0}",
                            bar.date.format(&Rfc3339).unwrap_or_else(|_| bar.date.to_string()),
                            bar.open, bar.high, bar.low, bar.close, bar.volume,
                        ));
                    }
                    lines.join("\n")
                }
                Err(e) => format!("Error: {e}"),
            }
        })
        .await
        .unwrap_or_else(|e| format!("Task error: {e}"))
    }

    #[tool(description = "Get account summary — balances, margin, buying power")]
    async fn account_summary(&self, Parameters(req): Parameters<AccountSummaryRequest>) -> String {
        let client = self.client.clone();
        tokio::task::spawn_blocking(move || {
            let group = AccountGroup(req.group.unwrap_or_else(|| "All".into()));
            let tags = &[
                AccountSummaryTags::NET_LIQUIDATION,
                AccountSummaryTags::TOTAL_CASH_VALUE,
                AccountSummaryTags::BUYING_POWER,
                AccountSummaryTags::GROSS_POSITION_VALUE,
                AccountSummaryTags::AVAILABLE_FUNDS,
                AccountSummaryTags::EXCESS_LIQUIDITY,
            ];

            match client.account_summary(&group, tags) {
                Ok(sub) => {
                    let mut lines = vec!["Account | Tag | Value | Currency".into(), "--- | --- | --- | ---".into()];
                    for item in sub.iter() {
                        match item {
                            AccountSummaryResult::Summary(v) => {
                                lines.push(format!("{} | {} | {} | {}", v.account, v.tag, v.value, v.currency));
                            }
                            AccountSummaryResult::End => break,
                        }
                    }
                    lines.join("\n")
                }
                Err(e) => format!("Error: {e}"),
            }
        })
        .await
        .unwrap_or_else(|e| format!("Task error: {e}"))
    }

    #[tool(description = "List all open positions in the account")]
    async fn positions(&self) -> String {
        let client = self.client.clone();
        tokio::task::spawn_blocking(move || match client.positions() {
            Ok(sub) => {
                let mut lines = vec!["Account | Symbol | Type | Exchange | Qty | Avg Cost".into(), "--- | --- | --- | --- | --- | ---".into()];
                for item in sub.iter() {
                    match item {
                        PositionUpdate::Position(p) => {
                            lines.push(format!(
                                "{} | {} | {} | {} | {} | {:.2}",
                                p.account, p.contract.symbol, p.contract.security_type,
                                p.contract.exchange, p.position, p.average_cost,
                            ));
                        }
                        PositionUpdate::PositionEnd => break,
                    }
                }
                lines.join("\n")
            }
            Err(e) => format!("Error: {e}"),
        })
        .await
        .unwrap_or_else(|e| format!("Task error: {e}"))
    }
}

// --- Helpers ---

fn parse_duration(s: &str) -> Duration {
    let s = s.trim().to_uppercase();
    let (num, unit) = s.split_at(s.len() - 1);
    let n: i32 = num.parse().unwrap_or(1);
    match unit {
        "D" => Duration::days(n),
        "W" => Duration::weeks(n),
        "M" => Duration::months(n),
        "Y" => Duration::years(n),
        _ => Duration::days(n),
    }
}

fn parse_bar_size(s: &str) -> BarSize {
    match s.to_lowercase().as_str() {
        "1s" => BarSize::Sec,
        "5s" => BarSize::Sec5,
        "15s" => BarSize::Sec15,
        "30s" => BarSize::Sec30,
        "1min" => BarSize::Min,
        "2min" => BarSize::Min2,
        "3min" => BarSize::Min3,
        "5min" => BarSize::Min5,
        "15min" => BarSize::Min15,
        "30min" => BarSize::Min30,
        "1h" => BarSize::Hour,
        "2h" => BarSize::Hour2,
        "3h" => BarSize::Hour3,
        "4h" => BarSize::Hour4,
        "8h" => BarSize::Hour8,
        "1d" => BarSize::Day,
        "1w" => BarSize::Week,
        "1m" => BarSize::Month,
        _ => BarSize::Day,
    }
}

fn parse_what_to_show(s: &str) -> WhatToShow {
    match s.to_lowercase().as_str() {
        "trades" => WhatToShow::Trades,
        "midpoint" => WhatToShow::MidPoint,
        "bid" => WhatToShow::Bid,
        "ask" => WhatToShow::Ask,
        "hvol" => WhatToShow::HistoricalVolatility,
        "ivol" => WhatToShow::OptionImpliedVolatility,
        _ => WhatToShow::Trades,
    }
}
