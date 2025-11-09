use crate::server::core::PomoServer;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, body::Incoming};
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use tokio::net::TcpListener;

pub struct HttpServer {
    server: Arc<PomoServer>,
}

impl HttpServer {
    pub fn new(server: PomoServer) -> Self {
        Self {
            server: Arc::new(server),
        }
    }

    pub async fn start(&self, addr: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(addr).await?;
        println!("🌐 HTTP server listening on {}", addr);

        loop {
            let (stream, _) = listener.accept().await?;
            let server = self.server.clone();

            tokio::spawn(async move {
                // let service = service_fn(move |_req: Request<Incoming>| async move {
                //     Ok::<_, hyper::Error>(
                //         Response::builder()
                //             .status(200)
                //             .header("Content-Type", "application/json")
                //             .body(Full::new(Bytes::from(r#"{"message": "HTTP works!"}"#)))
                //             .unwrap(),
                //     )
                // });
                let service = service_fn(move |req: Request<Incoming>| {
                    let server = server.clone();
                    async move { handle_http_request(req, server).await }
                });

                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await;
            });
        }
    }
}

async fn handle_http_request(
    req: Request<Incoming>,
    server: Arc<PomoServer>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let method = req.method();
    let path = req.uri().path();

    let (status, body) = match (method, path) {
        (&hyper::Method::GET, "/ping") => {
            server.process_request(crate::protocol::Request::Ping).await;
            (200, r#"{"message": "pong"}"#.to_string())
        }
        (&hyper::Method::GET, "/timer/status") => {
            let response = server
                .process_request(crate::protocol::Request::GetStatus)
                .await;
            (200, serde_json::to_string(&response).unwrap())
        }
        (&hyper::Method::POST, "/timer/start") => {
            server
                .process_request(crate::protocol::Request::Start)
                .await;
            (200, r#"{"success": true}"#.to_string())
        }
        (&hyper::Method::POST, "/timer/pause") => {
            server
                .process_request(crate::protocol::Request::Pause)
                .await;
            (200, r#"{"success": true}"#.to_string())
        }
        (&hyper::Method::POST, "/timer/resume") => {
            server
                .process_request(crate::protocol::Request::Resume)
                .await;
            (200, r#"{"success": true}"#.to_string())
        }
        (&hyper::Method::POST, "/timer/reset") => {
            server
                .process_request(crate::protocol::Request::Reset)
                .await;
            (200, r#"{"success": true}"#.to_string())
        }
        (&hyper::Method::POST, "/timer/switch") => {
            server
                .process_request(crate::protocol::Request::SwitchMode)
                .await;
            (200, r#"{"success": true}"#.to_string())
        }
        // (&hyper::Method::PUT, "/timer/task") => { ... }  // Need to parse JSON body
        // (&hyper::Method::PUT, "/timer/preset") => { ... }  // Need to parse JSON body
        _ => (404, r#"{"error": "Not Found"}"#.to_string()),
    };

    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}
