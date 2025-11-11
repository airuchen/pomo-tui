use anyhow::{Ok, Result};
use std::time::Duration;

use crate::protocol::{Request, Response};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Default)]
pub struct PomoClient {
    reader: Option<BufReader<tokio::net::tcp::OwnedReadHalf>>,
    writer: Option<BufWriter<tokio::net::tcp::OwnedWriteHalf>>,
}

impl PomoClient {
    pub fn new() -> Self {
        Self {
            reader: None,
            writer: None,
        }
    }

    pub async fn connect(&mut self, addr: &str) -> Result<()> {
        let stream = TcpStream::connect(addr).await?;

        // Note: Split the stream into read/write halves
        let (read_half, write_half) = stream.into_split();

        self.reader = Some(BufReader::new(read_half));
        self.writer = Some(BufWriter::new(write_half));

        Ok(())
    }

    pub async fn send_request(&mut self, request: Request) -> Result<Response> {
        let reader = self
            .reader
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Reader not connected"))?;
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Writer not connected"))?;

        // Serialize request to JSON
        let request_json = serde_json::to_string(&request)?;

        // Send JSON request with newline
        writer.write_all(request_json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;

        // Read JSON response
        let mut response_line = String::new();
        let bytes_read =
            timeout(Duration::from_secs(5), reader.read_line(&mut response_line)).await??;

        if bytes_read == 0 {
            return Err(anyhow::anyhow!("Connection closed by server"));
        }

        // Parse JSON response
        let response: Response = serde_json::from_str(response_line.trim())?;

        Ok(response)
    }

    // Convenience functions
    pub async fn set_task_name(&mut self, task_name: String) -> Result<()> {
        self.send_request(Request::SetTask(task_name)).await?;
        Ok(())
    }

    pub async fn reset(&mut self) -> Result<()> {
        self.send_request(Request::Reset).await?;
        Ok(())
    }

    pub async fn get_status(&mut self) -> Result<Response> {
        self.send_request(Request::GetStatus).await
    }
}
