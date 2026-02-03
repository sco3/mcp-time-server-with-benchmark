mod rpc;
mod types;
mod tools;
mod handlers;

use axum::routing::post;
use axum::{Router};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use handlers::mcp_handler;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the TLS certificate file
    #[arg(long)]
    tls_cert: Option<PathBuf>,
    /// Path to the TLS key file
    #[arg(long)]
    tls_key: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Build our application with routes for both /mcp and /mcp/
    let app = Router::new()
        .route("/mcp", post(mcp_handler))
        .route("/mcp/", post(mcp_handler));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    match (args.tls_cert, args.tls_key) {
        (Some(cert_path), Some(key_path)) => {
            println!("MCP server listening on https://{addr}");
            let config = RustlsConfig::from_pem_file(cert_path, key_path)
                .await
                .unwrap_or_else(|e| {
                    eprintln!("[ERROR] Failed to load TLS certificate/key: {e}");
                    std::process::exit(1);
                });
            axum_server::bind_rustls(addr, config)
                .serve(app.into_make_service())
                .await
                .unwrap_or_else(|e| {
                    eprintln!("[ERROR] Failed to start HTTPS server: {e}");
                    std::process::exit(1);
                });
        }
        (None, None) => {
            println!("MCP server listening on http://{addr}");
            let listener = tokio::net::TcpListener::bind(addr)
                .await
                .unwrap_or_else(|e| {
                    eprintln!("[ERROR] Failed to bind to address {addr}: {e}");
                    std::process::exit(1);
                });
            axum::serve(listener, app).await.unwrap_or_else(|e| {
                eprintln!("[ERROR] Failed to start HTTP server: {e}");
                std::process::exit(1);
            });
        }
        _ => {
            eprintln!(
                "[ERROR] Both --tls-cert and --tls-key must be provided together to enable TLS."
            );
            std::process::exit(1);
        }
    }
}
