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
const IBKR_PORTS: &[u16] = &[4002, 4001, 7497, 7496];

#[derive(Parser)]
#[command(name = "ibkr-mcp", about = "Read-only IBKR MCP server")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// TWS/Gateway address (if omitted, tries ports 4002, 4001, 7497, 7496)
    #[arg(long)]
    ibkr_addr: Option<String>,

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

    let shared_client = Arc::new(match &cli.ibkr_addr {
        Some(addr) => connect_with_fallback(addr, cli.client_id)?,
        None => connect_auto_discover(cli.client_id)?,
    });

    tracing::info!("Connected to IBKR (client ID {})", shared_client.client_id());

    let config = StreamableHttpServerConfig::default();
    let cancel = config.cancellation_token.clone();

    let service = StreamableHttpService::new(
        move || Ok(server::IbkrServer::new(shared_client.clone())),
        Arc::new(LocalSessionManager::default()),
        config,
    );

    let app = axum::Router::new().nest_service("/mcp", service);
    let addr = format!("127.0.0.1:{}", cli.port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            tracing::info!("ibkr-mcp already running on {addr}");
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

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

fn connect_auto_discover(preferred_id: i32) -> Result<Client> {
    for port in IBKR_PORTS {
        let addr = format!("127.0.0.1:{port}");
        match connect_with_fallback(&addr, preferred_id) {
            Ok(c) => {
                tracing::info!("Found TWS/Gateway on port {port}");
                return Ok(c);
            }
            Err(_) => continue,
        }
    }
    anyhow::bail!(
        "No TWS/Gateway found on ports {:?} (tried client IDs {preferred_id}-{})",
        IBKR_PORTS, preferred_id + 2
    )
}

fn connect_with_fallback(addr: &str, preferred_id: i32) -> Result<Client> {
    for id in [preferred_id, preferred_id + 1, preferred_id + 2] {
        match Client::connect(addr, id) {
            Ok(c) => return Ok(c),
            Err(e) => {
                tracing::warn!("Client ID {id} failed on {addr}: {e}, trying next...");
            }
        }
    }
    anyhow::bail!("Failed to connect to TWS/Gateway at {addr} (tried client IDs {preferred_id}-{})", preferred_id + 2)
}
