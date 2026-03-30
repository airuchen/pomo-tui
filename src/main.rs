// Copyright (c) 2025 Yu-Wen Chen
// Licensed under the MIT License (see LICENSE file)

mod client;
mod db;
mod logging;
mod protocol;
mod server;
mod timer;
mod todo;
mod tui;
mod utils;

use crate::client::tcp::PomoClient;
use crate::server::core::PomoServer;
use crate::server::http::HttpServer;
use crate::server::tcp::TcpServer;
use crate::tui::ServerApp;
use anyhow::Result;
use clap::Parser;
use sqlx::SqlitePool;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "pomo-tui")]
#[command(author, version, about = "A Pomodoro TUI app")]
struct Args {
    #[arg(long)]
    server: bool,

    #[arg(long, default_value = "127.0.0.1:1880")]
    tcp_addr: String,

    #[arg(long, default_value = "127.0.0.1:1881")]
    http_addr: String,
}

async fn spawn_servers(
    tcp_addr: &str,
    http_addr: &str,
    pool: SqlitePool,
) -> (
    tokio::task::JoinHandle<Result<()>>,
    tokio::task::JoinHandle<Result<()>>,
) {
    let pomo_server = Arc::new(PomoServer::new(pool.clone()));
    let tcp_server = TcpServer::new(pomo_server.clone());
    let http_server = HttpServer::new(pomo_server, pool);

    let tcp_addr = tcp_addr.to_string();
    let http_addr = http_addr.to_string();

    let tcp_task = tokio::spawn(async move { tcp_server.start(&tcp_addr).await });
    let http_task = tokio::spawn(async move { http_server.start(&http_addr).await });
    (tcp_task, http_task)
}

async fn start_network_tui(tcp_addr: &str, pool: SqlitePool) -> Result<()> {
    let mut client = PomoClient::new();
    client.connect(tcp_addr).await?;

    let mut terminal = ratatui::init();
    let mut app = ServerApp::new(client, Some(pool));
    app.run(&mut terminal).await?;
    ratatui::restore();

    Ok(())
}

async fn start_embedded_server_and_tui(
    tcp_addr: &str,
    http_addr: &str,
    pool: SqlitePool,
) -> Result<()> {
    let (tcp_server, http_server) = spawn_servers(tcp_addr, http_addr, pool.clone()).await;

    // Give servers time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let mut client = PomoClient::new();
    client.connect(tcp_addr).await?;

    let mut terminal = ratatui::init();
    let mut app = ServerApp::new(client, Some(pool.clone()));
    let _ = app.run(&mut terminal).await;

    ratatui::restore();
    tcp_server.abort();
    http_server.abort();

    Ok(())
}

async fn server_exists(tcp_addr: &str) -> bool {
    match tokio::net::TcpStream::connect(tcp_addr).await {
        Ok(_) => true,
        Err(_) => false,
    }
}

async fn start_server(tcp_addr: &str, http_addr: &str, pool: SqlitePool) -> Result<()> {
    let (mut tcp_server, mut http_server) = spawn_servers(tcp_addr, http_addr, pool).await;

    // Wait until one server exits or we receive a shutdown signal.
    tokio::select! {
        r = &mut tcp_server => {
            r??; // JoinError? then anyhow::Error?
        }
        r = &mut http_server => {
            r??;
        }
        _ = tokio::signal::ctrl_c() => {
            println!("Shutdown signal received");
        }
    }

    // Stop both tasks (if still running).
    tcp_server.abort();
    http_server.abort();

    // Optional: ensure abort is observed (avoid noisy "task was cancelled" later)
    let _ = tcp_server.await;
    let _ = http_server.await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let db_path = dirs::data_local_dir()
        .map(|p| p.join("pomo-tui/pomo.db"))
        .or_else(|| dirs::home_dir().map(|p| p.join(".pomo-tui/pomo.db")))
        .ok_or_else(|| anyhow::anyhow!("Cannot determine data directory"))?;

    let pool = db::init(&db_path).await?;

    if args.server {
        println!("Starting Pomo server");
        start_server(&args.tcp_addr, &args.http_addr, pool).await
    } else {
        if server_exists(&args.tcp_addr).await {
            println!("Connecting to existing server ...");
            start_network_tui(&args.tcp_addr, pool).await
        } else {
            println!("Starting embedded server and TUI");
            start_embedded_server_and_tui(&args.tcp_addr, &args.http_addr, pool).await
        }
    }
}
