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
use std::env;
use std::sync::Arc;

const TCP_ADDR: &str = "127.0.0.1:1880";
const HTTP_ADDR: &str = "127.0.0.1:1881";

async fn spawn_servers() -> (
    tokio::task::JoinHandle<Result<()>>,
    tokio::task::JoinHandle<Result<()>>,
) {
    let pomo_server = Arc::new(PomoServer::new());
    let tcp_server = TcpServer::new(pomo_server.clone());
    let http_server = HttpServer::new(pomo_server);

    let tcp_task = tokio::spawn(async move { tcp_server.start(TCP_ADDR).await });
    let http_task = tokio::spawn(async move { http_server.start(HTTP_ADDR).await });
    (tcp_task, http_task)
}

async fn start_network_tui() -> Result<()> {
    let mut client = PomoClient::new();
    client.connect(TCP_ADDR).await?;

    let mut terminal = ratatui::init();
    let mut app = ServerApp::new(client);
    app.run(&mut terminal).await?;
    ratatui::restore();

    Ok(())
}

async fn start_embedded_server_and_tui() -> Result<()> {
    let (tcp_server, http_server) = spawn_servers().await;

    // Give servers time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let mut client = PomoClient::new();
    client.connect(TCP_ADDR).await?;

    let mut terminal = ratatui::init();
    let mut app = ServerApp::new(client);
    let _ = app.run(&mut terminal).await;

    ratatui::restore();
    tcp_server.abort();
    http_server.abort();

    Ok(())
}

async fn server_exists() -> bool {
    match tokio::net::TcpStream::connect(TCP_ADDR).await {
        Ok(_) => true,
        Err(_) => false,
    }
}

async fn start_server() -> Result<()> {
    let (tcp_server, http_server) = spawn_servers().await;

    tokio::signal::ctrl_c().await?;
    tcp_server.abort();
    http_server.abort();

    Ok(())
}

fn print_help() {
    println!("PomoTUI");
    println!();
    println!("USAGE:");
    println!("    pomo                     # TUI mode");
    println!("    pomo --server            # Server only (daemon mode)");
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("--help") => {
            print_help();
            Ok(())
        }
        Some("--server") => {
            println!("Starting Pomo server");
            start_server().await
        }

        _ => {
            if server_exists().await {
                println!("Connecting to existing server ...");
                start_network_tui().await
            } else {
                println!("Starting embedded server and TUI");
                start_embedded_server_and_tui().await
            }
        }
    }
}
