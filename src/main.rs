mod client;
mod logging;
mod protocol;
mod server;
mod timer;
mod tui;
mod utils;

use crate::client::tcp::PomoClient;
use crate::server::core::PomoServer;
use crate::server::http::HttpServer;
use crate::server::tcp::TcpServer;
use crate::tui::ServerApp;
use anyhow::Result;
use clap::Parser;
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
) -> (
    tokio::task::JoinHandle<Result<()>>,
    tokio::task::JoinHandle<Result<()>>,
) {
    let pomo_server = Arc::new(PomoServer::new());
    let tcp_server = TcpServer::new(pomo_server.clone());
    let http_server = HttpServer::new(pomo_server);

    let tcp_addr = tcp_addr.to_string();
    let http_addr = http_addr.to_string();

    let tcp_task = tokio::spawn(async move { tcp_server.start(&tcp_addr).await });
    let http_task = tokio::spawn(async move { http_server.start(&http_addr).await });
    (tcp_task, http_task)
}

async fn start_network_tui(tcp_addr: &str) -> Result<()> {
    let mut client = PomoClient::new();
    client.connect(tcp_addr).await?;

    let mut terminal = ratatui::init();
    let mut app = ServerApp::new(client);
    app.run(&mut terminal).await?;
    ratatui::restore();

    Ok(())
}

async fn start_embedded_server_and_tui(tcp_addr: &str, http_addr: &str) -> Result<()> {
    let (tcp_server, http_server) = spawn_servers(tcp_addr, http_addr).await;

    // Give servers time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let mut client = PomoClient::new();
    client.connect(tcp_addr).await?;

    let mut terminal = ratatui::init();
    let mut app = ServerApp::new(client);
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

async fn start_server(tcp_addr: &str, http_addr: &str) -> Result<()> {
    let (tcp_server, http_server) = spawn_servers(tcp_addr, http_addr).await;

    tokio::signal::ctrl_c().await?;
    tcp_server.abort();
    http_server.abort();

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.server {
        println!("Starting Pomo server");
        start_server(&args.tcp_addr, &args.http_addr).await
    } else {
        if server_exists(&args.tcp_addr).await {
            println!("Connecting to existing server ...");
            start_network_tui(&args.tcp_addr).await
        } else {
            println!("Starting embedded server and TUI");
            start_embedded_server_and_tui(&args.tcp_addr, &args.http_addr).await
        }
    }
}
