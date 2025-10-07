mod logging;
mod protocol;
mod server;
mod timer;
mod tui;
mod utils;

use crate::server::core::PomoServer;
use std::env;
use std::io;
use tui::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("--server") => {
            println!("Starting Pomo server");
            start_server().await
        }
        Some("--help") => {
            print_help();
            Ok(())
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

async fn start_server() -> Result<(), Box<dyn std::error::Error>> {
    let server = PomoServer::new();
    println!("Server started");
    tokio::signal::ctrl_c().await?;
    Ok(())
}

fn print_help() {
    println!("PomoTUI");
    println!();
    println!("USAGE:");
    println!("    pomo                     # TUI mode");
    println!("    pomo --server            # Server only (daemon mode)");
}
