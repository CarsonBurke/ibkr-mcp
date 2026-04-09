mod server;

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use ibapi::client::blocking::Client;
use rmcp::transport::StreamableHttpServerConfig;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::StreamableHttpService;
use tracing_subscriber::{self, EnvFilter};

const DEFAULT_PORT: u16 = 3099;
const DEFAULT_IBKR_ADDR: &str = "127.0.0.1:4002";

#[derive(Parser)]
#[command(name = "ibkr-mcp", about = "Read-only IBKR MCP server")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// TWS/Gateway address
    #[arg(long, default_value = DEFAULT_IBKR_ADDR)]
    ibkr_addr: String,

    /// IBKR client ID
    #[arg(long, default_value_t = 99)]
    client_id: i32,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let cli = Cli::parse();

    let client = Client::connect(&cli.ibkr_addr, cli.client_id)
        .map_err(|e| anyhow::anyhow!("Failed to connect to TWS/Gateway at {}: {e}", cli.ibkr_addr))?;
    let shared_client = Arc::new(client);

    tracing::info!("Connected to IBKR at {} (client ID {})", cli.ibkr_addr, cli.client_id);

    let config = StreamableHttpServerConfig::default();
    let cancel = config.cancellation_token.clone();

    let service = StreamableHttpService::new(
        move || Ok(server::IbkrServer::new(shared_client.clone())),
        Arc::new(LocalSessionManager::default()),
        config,
    );

    let app = axum::Router::new().nest_service("/mcp", service);
    let addr = format!("127.0.0.1:{}", cli.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("ibkr-mcp listening on http://{addr}/mcp");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Shutting down");
            cancel.cancel();
        })
        .await?;

    Ok(())
}
