mod logging;
mod protocol;
mod server;
mod timer;
mod tui;
mod utils;

use crate::server::core::PomoServer;
use crate::server::http::HttpServer;
use crate::server::tcp::TcpServer;
use anyhow::Result;
use pomo_tui::client::tcp::PomoClient;
use std::env;
use tui::App;

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

        Some("--experimental") => {
            if server_exists().await {
                println!("Connecting to existing server ...");
                // start_network_tui().awiat()
                todo!()
            } else {
                println!("Starting embedded server and TUI");
                // start_embedded_server_and_tui().await()
                todo!()
            }
        }

        _ => {
            let mut terminal = ratatui::init();
            let app_result = App::new().run(&mut terminal);
            ratatui::restore();
            app_result?;
            Ok(())
        }
    }
}

async fn server_exists() -> bool {
    match tokio::net::TcpStream::connect("127.0.0.1:1880").await {
        Ok(_) => true,
        Err(_) => false,
    }
}

async fn start_server() -> Result<()> {
    let pomo_server = PomoServer::new();
    let tcp_server = TcpServer::new(pomo_server.clone());
    let http_server = HttpServer::new(pomo_server);

    let tcp_task = tokio::spawn(async move { tcp_server.start("127.0.0.1:1880").await });
    let http_task = tokio::spawn(async move { http_server.start("127.0.0.1:1881").await });

    // Test client
    {
        let mut client = PomoClient::new();
        client.connect("127.0.0.1:1880").await?;
        let response = client
            .send_request(pomo_tui::protocol::Request::Start)
            .await?;
        println!("response: {:?}", response);
    }

    println!("Server started");

    tokio::signal::ctrl_c().await?;
    tcp_task.abort();
    http_task.abort();
    println!("Server stopped");
    Ok(())
}

fn print_help() {
    println!("PomoTUI");
    println!();
    println!("USAGE:");
    println!("    pomo                     # TUI mode");
    println!("    pomo --server            # Server only (daemon mode)");
}
