// Copyright (c) 2025 Yu-Wen Chen
// Licensed under the MIT License (see LICENSE file)

use crate::protocol::{Request, Response};
use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::server::core::PomoServer;

pub struct TcpServer {
    server: Arc<PomoServer>,
}

impl TcpServer {
    pub fn new(server: Arc<PomoServer>) -> Self {
        Self { server }
    }

    pub async fn start(&self, addr: &str) -> Result<()> {
        // implement TCP listener
        let listener = TcpListener::bind(addr).await?;
        eprintln!("Pomo TcpServer listening on {}", addr);

        // Accept connections in a loop
        loop {
            let (stream, _) = listener.accept().await?;
            eprintln!("New client connected");

            let server = Arc::clone(&self.server);
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection_static(server, stream).await {
                    eprintln!("Error handling connection: {}", e);
                }
            });
        }

        // This line is never reached
        // Ok(())
    }

    async fn handle_connection_static(server: Arc<PomoServer>, stream: TcpStream) -> Result<()> {
        let (read_half, mut write_half) = stream.into_split();
        // create BufReader for line-based reading
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();

        loop {
            line.clear();
            // Read JSON requests line by line
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    eprintln!("Client disconnected");
                    break;
                }
                Ok(_) => {
                    let request_str = line.trim();

                    // Deserialize to Request
                    match serde_json::from_str::<Request>(request_str) {
                        Ok(request) => {
                            let response = server.process_request(request).await;

                            // Send JSON response back
                            let response_json = serde_json::to_string(&response)?;
                            write_half.write_all(response_json.as_bytes()).await?;
                            write_half.write_all(b"\n").await?;
                        }
                        Err(e) => {
                            eprintln!("Invalid JSON request: {}", e);
                            let error_response = Response::Error(format!("Invalid JSON: {}", e));
                            let error_json = serde_json::to_string(&error_response)?;
                            write_half.write_all(error_json.as_bytes()).await?;
                            write_half.write_all(b"\n").await?;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading from client: {}", e);
                    break;
                }
            }
        }
        Ok(())
    }
}
