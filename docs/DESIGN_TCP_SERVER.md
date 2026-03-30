# PomoServer: Smart Local Server Architecture

## Overview

This document outlines a **Smart Local Server** architecture for personal Pomodoro timer use. The design provides seamless client-server functionality with automatic server management, dual protocol support (TCP + HTTP), and graceful fallbacks - optimized for single-user productivity workflows.

## Design Philosophy

### **Invisible Complexity**
- **Just works**: User runs `pomo` and gets a working timer
- **Auto-management**: Server spawns automatically when needed
- **Graceful fallbacks**: Network issues don't break functionality
- **Personal optimized**: Single-user, local-only by default

### **Dual Protocol Strategy**
- **TCP**: For TUI clients (persistent, low-latency, real-time)
- **HTTP**: For scripts/integrations (stateless, standard, tooling-friendly)
- **Unified backend**: Same async timer logic serves both protocols
- **Smart routing**: Right protocol for the right use case

### **Async Architecture Benefits**
- **True concurrency**: Multiple clients don't block each other
- **Efficient resource usage**: Tasks yield CPU when waiting for locks
- **Better throughput**: Higher performance under concurrent load
- **Future-proof**: Easy to add timeouts, retries, and advanced features

## Current State vs Target Architecture

### Current Architecture (Monolithic)
```
┌─────────────────────────────────┐
│        Monolithic TUI           │
│  ┌─────────┬─────────┬────────┐ │
│  │  Timer  │   UI    │ Logging│ │
│  └─────────┴─────────┴────────┘ │
│              Local Files        │
└─────────────────────────────────┘
```
ascii art
### Target Architecture (Smart Local Server)
```
                    ┌──────────────────────────────────┐
                    │        Pomo Server               │
                    │     (Auto-spawned/Daemon)        │
                    │                                  │
                    │ ┌──────────────────────────────┐ │
                    │ │          Timer Logic         │ │
                    │ └──────────────────────────────┘ │
                    │                                  │
                    │ ┌─────────────┐ ┌──────────────┐ │
                    │ │ TCP Server  │ │ HTTP Server  │ │
                    │ │   :1888     │ │    :1889     │ │
                    │ │ (TUI Proto) │ │ (REST API)   │ │
                    │ └─────────────┘ └──────────────┘ │
                    └─────────┬─────────────┬──────────┘
                              │             │
                    ┌─────────┼─────────────┼──────────┐
                    │         │             │          │
            ┌───────▼──────┐  │   ┌─────────▼──────┐   │
            │ TUI Client 1 │  │   │   Scripts      │   │
            │              │  │   │   Waybar       │   │
            │ ┌──────────┐ │  │   │   Polybar      │   │
            │ │    UI    │ │  │   │   curl/wget    │   │
            │ │Rendering │ │  │   └────────────────┘   │
            │ └──────────┘ │  │                        │
            │ ┌──────────┐ │  │   ┌────────────────┐   │
            │ │TCP Client│ │  │   │  Mobile App    │   │
            │ │(Persist) │ │  │   │  Web Dashboard │   │
            │ └──────────┘ │  │   │  (Future)      │   │
            └──────────────┘  │   └────────────────┘   │
                              │                        │
            ┌─────────────────▼┐                       │
            │ TUI Client 2     │                       │
            │ (Same timer      │                       │
            │  state shared)   │                       │
            └──────────────────┘                       │
                                                       │
            ┌──────────────────────────────────────────▼┐
            │           Fallback Mode                   │
            │  (If networking fails, run standalone)    │
            └───────────────────────────────────────────┘
```

## Project Structure

### Updated Directory Structure
```
src/
├── main.rs              # Smart startup with auto-server detection
├── timer.rs             # Existing timer logic (minimal changes)
├── tui.rs               # TUI client (network-based, fallback capable)
├── utils.rs             # Existing utilities (unchanged)
├── logging.rs           # Server-side logging (unchanged)
├── server/              # NEW: Dual-protocol server (~300 lines total)
│   ├── mod.rs          # Module exports and startup logic
│   ├── tcp.rs          # TCP server for TUI clients (~100 lines)
│   ├── http.rs         # HTTP server for scripts/APIs (~100 lines)
│   └── core.rs         # Shared server logic (~100 lines)
├── client/              # NEW: Network client (~150 lines total)
│   ├── mod.rs          # Client interface
│   ├── tcp.rs          # TCP client implementation (~100 lines)
│   └── cache.rs        # Status caching for UI responsiveness (~50 lines)
└── protocol/            # NEW: Shared message types (~100 lines total)
    ├── mod.rs          # Protocol exports
    ├── messages.rs     # Request/Response enums (~50 lines)
    └── http.rs         # HTTP endpoint definitions (~50 lines)
```

### Updated Cargo.toml
```toml
[package]
name = "pomo-server"
version = "0.2.0"
edition = "2021"
description = "Smart local Pomodoro timer with dual-protocol server"

[[bin]]
name = "pomo"
path = "src/main.rs"

[dependencies]
# Existing dependencies (unchanged)
crossterm = "0.29.0"
ratatui = "0.30.0-alpha.5"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
notify-rust = "4.0"
log = "0.4"

# NEW: Async networking
tokio = { version = "1.0", features = ["full"] }

# NEW: HTTP server (lightweight)
hyper = { version = "1.0", features = ["full"] }
hyper-util = "0.1"
http-body-util = "0.1"
```

## Protocol Design

### Simple Message Types (`src/bridge/protocol.rs`)
```rust
use serde::{Deserialize, Serialize};

/// Simple request types - covers all basic timer operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    // Timer control
    Start,
    Pause,
    Resume,
    Reset,
    SwitchMode,
    
    // Configuration
    SetPreset(String),      // "short", "long", "test"
    SetTask(String),
    
    // Queries
    GetStatus,
    Ping,
}

/// Simple response types - minimal but complete
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Ok,
    Status {
        mode: String,           // "Work 👨‍💻" or "Break ☕"
        remaining_secs: u64,
        is_running: bool,
        is_paused: bool,
        is_idle: bool,
        task: String,
        preset: String,         // "short", "long", "test"
    },
    Error(String),
    Pong,
}
```

### Protocol Details
- **Transport**: Raw TCP sockets
- **Format**: JSON per line (newline-delimited)
- **Encoding**: UTF-8
- **No authentication**: Trust local network
- **No encryption**: Plain text (can add TLS later)

### Example Protocol Exchange
```bash
# Client connects to server:8080
Client → Server: {"GetStatus":null}
Server → Client: {"Status":{"mode":"Work 👨‍💻","remaining_secs":1500,"is_running":false,"is_paused":false,"is_idle":true,"task":"","preset":"short"}}

Client → Server: {"SetTask":"Write documentation"}
Server → Client: "Ok"

Client → Server: {"Start":null}
Server → Client: "Ok"

Client → Server: {"GetStatus":null}
Server → Client: {"Status":{"mode":"Work 👨‍💻","remaining_secs":1498,"is_running":true,"is_paused":false,"is_idle":false,"task":"Write documentation","preset":"short"}}
```

## Server Implementation

### Bridge Server (`src/bridge/server.rs`)
```rust
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use std::sync::{Arc, Mutex};
use crate::{timer::Timer, bridge::protocol::{Request, Response}};

/// Simple TCP server that shares a single Timer instance
pub struct BridgeServer {
    /// Shared timer state - thread-safe with Arc<Mutex<>>
    timer: Arc<Mutex<Timer>>,
    /// Server bind address
    addr: String,
}

impl BridgeServer {
    pub fn new(addr: String) -> Self {
        Self {
            timer: Arc::new(Mutex::new(Timer::new())),
            addr,
        }
    }

    /// Run the server - accepts connections and spawns handlers
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        println!("🚀 PomoBridge server listening on {}", self.addr);
        println!("💡 Test with: echo '{{\"GetStatus\":null}}' | nc {}", self.addr.replace("0.0.0.0", "127.0.0.1"));
        
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    println!("🔗 Client connected: {}", addr);
                    let timer = self.timer.clone();
                    
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, timer).await {
                            eprintln!("❌ Client error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("❌ Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// Handle individual client connection
    async fn handle_client(
        stream: TcpStream,
        timer: Arc<Mutex<Timer>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut reader = BufReader::new(&stream);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await? {
                0 => {
                    println!("📱 Client disconnected");
                    break;
                }
                _ => {
                    let request: Request = match serde_json::from_str(line.trim()) {
                        Ok(req) => req,
                        Err(e) => {
                            let error_response = Response::Error(format!("Invalid JSON: {}", e));
                            Self::send_response(&stream, error_response).await?;
                            continue;
                        }
                    };

                    let response = Self::process_request(&timer, request);
                    Self::send_response(&stream, response).await?;
                }
            }
        }
        Ok(())
    }

    /// Process a client request and return appropriate response
    fn process_request(timer: &Arc<Mutex<Timer>>, request: Request) -> Response {
        let mut timer = match timer.lock() {
            Ok(t) => t,
            Err(_) => return Response::Error("Server error: timer lock poisoned".to_string()),
        };

        match request {
            Request::Start => {
                if timer.is_idle() || timer.is_paused() {
                    timer.toggle();
                }
                Response::Ok
            }
            Request::Pause => {
                if timer.is_running() && !timer.is_paused() {
                    timer.toggle();
                }
                Response::Ok
            }
            Request::Resume => {
                if timer.is_paused() {
                    timer.toggle();
                }
                Response::Ok
            }
            Request::Reset => {
                timer.reset();
                Response::Ok
            }
            Request::SwitchMode => {
                timer.switch_mode();
                Response::Ok
            }
            Request::SetTask(name) => {
                timer.set_task_name(&name);
                Response::Ok
            }
            Request::SetPreset(preset) => {
                use crate::timer::Preset;
                let preset_enum = match preset.as_str() {
                    "short" => Preset::Short,
                    "long" => Preset::Long,
                    "test" => Preset::Test,
                    _ => return Response::Error(format!("Unknown preset: {}", preset)),
                };
                timer.set_preset(preset_enum);
                Response::Ok
            }
            Request::GetStatus => {
                Response::Status {
                    mode: timer.get_mode().to_string(),
                    remaining_secs: timer.get_remaining().as_secs(),
                    is_running: timer.is_running(),
                    is_paused: timer.is_paused(),
                    is_idle: timer.is_idle(),
                    task: timer.get_task_name().to_string(),
                    preset: format!("{:?}", timer.get_preset()).to_lowercase(),
                }
            }
            Request::Ping => Response::Pong,
        }
    }

    /// Send a response back to the client
    async fn send_response(
        mut stream: &TcpStream,
        response: Response,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(&response)?;
        stream.write_all((json + "\n").as_bytes()).await?;
        Ok(())
    }
}
```

### Timer Updates (Minimal Changes)

#### Add getter for preset (`src/timer.rs`)
```rust
impl Timer {
    // Add this method to existing Timer implementation
    pub fn get_preset(&self) -> Preset {
        self.timeset
    }
}
```

## Client Implementation

### Optional TCP Client (`src/bridge/client.rs`)
```rust
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use crate::bridge::protocol::{Request, Response};

/// Simple TCP client for connecting to BridgeServer
pub struct BridgeClient {
    stream: Option<TcpStream>,
    addr: String,
}

impl BridgeClient {
    pub fn new(addr: String) -> Self {
        Self {
            stream: None,
            addr,
        }
    }

    /// Connect to the bridge server
    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let stream = TcpStream::connect(&self.addr).await?;
        self.stream = Some(stream);
        Ok(())
    }

    /// Send a request and get a response
    pub async fn send_request(&mut self, request: Request) -> Result<Response, Box<dyn std::error::Error>> {
        let stream = self.stream.as_mut()
            .ok_or("Not connected to server")?;

        // Send request
        let json = serde_json::to_string(&request)?;
        stream.write_all((json + "\n").as_bytes()).await?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        
        let response: Response = serde_json::from_str(line.trim())?;
        Ok(response)
    }
}
```

## Entry Point Implementation

### Smart Server Detection & Spawning (`src/main.rs`)
```rust
mod timer;
mod tui;
mod utils;
mod logging;
mod server;
mod client;

use std::env;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};

const SERVER_ADDR: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    match args.get(1).map(|s| s.as_str()) {
        Some("--server") => {
            // Pure server mode (no TUI)
            println!("🚀 Starting Pomo server (daemon mode)...");
            start_server().await
        }
        Some("--client") => {
            // Pure client mode (server must exist)
            println!("📡 Starting TUI client...");
            start_tui_client().await
        }
        Some("--help") => {
            print_help();
            Ok(())
        }
        _ => {
            // Auto mode: detect server or spawn both
            if server_exists().await {
                println!("📡 Connecting to existing server...");
                start_tui_client().await
            } else {
                println!("🚀 Starting server + TUI client...");
                spawn_server_and_client().await
            }
        }
    }
}

/// Check if server is already running
async fn server_exists() -> bool {
    TcpStream::connect(SERVER_ADDR).await.is_ok()
}

/// Start server only
async fn start_server() -> Result<(), Box<dyn std::error::Error>> {
    let server = server::PomoServer::new(SERVER_ADDR.to_string());
    server.run().await
}

/// Start TUI client only (assumes server exists)
async fn start_tui_client() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();
    let app_result = tui::App::new(SERVER_ADDR.to_string()).await?.run(&mut terminal).await;
    ratatui::restore();
    app_result
}

/// Spawn embedded server + TUI client
async fn spawn_server_and_client() -> Result<(), Box<dyn std::error::Error>> {
    // Spawn server in background task
    let server = server::PomoServer::new(SERVER_ADDR.to_string());
    tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("❌ Server error: {}", e);
        }
    });
    
    // Wait for server to start
    for _ in 0..10 {
        sleep(Duration::from_millis(100)).await;
        if server_exists().await {
            break;
        }
    }
    
    // Start TUI client
    start_tui_client().await
}

fn print_help() {
    println!("PomoServer - Client-Server Pomodoro Timer");
    println!();
    println!("USAGE:");
    println!("    pomo                     # Auto: connect to server or spawn both");
    println!("    pomo --server            # Start server only (daemon mode)");
    println!("    pomo --client            # Start TUI client only");
    println!("    pomo --help              # Show this help");
    println!();
    println!("EXAMPLES:");
    println!("    # Terminal 1: Start server");
    println!("    pomo --server");
    println!();
    println!("    # Terminal 2: Connect TUI client");
    println!("    pomo --client");
    println!();
    println!("    # Terminal 3: API testing");
    println!("    echo '{{\"GetStatus\":null}}' | nc 127.0.0.1 8080");
    println!("    echo '{{\"Start\":null}}' | nc 127.0.0.1 8080");
}
```

## TUI Client Transformation

### Network-Based TUI (`src/tui.rs`)
The TUI needs significant refactoring to work as a network client:

```rust
use crate::client::PomoClient;
use crate::server::protocol::{Request, Response, TimerStatus};

pub struct App {
    // Replace direct timer with network client
    client: PomoClient,
    cached_status: TimerStatus,
    last_update: Instant,
    
    // Existing UI state (unchanged)
    exit: bool,
    show_hint: bool,
    app_mode: AppMode,
    task_input: TaskInput,
}

impl App {
    /// Create new TUI client connected to server
    pub async fn new(server_addr: String) -> Result<Self, Box<dyn std::error::Error>> {
        let mut client = PomoClient::new(server_addr);
        client.connect().await?;
        
        // Get initial status
        let status = client.get_status().await?;
        
        Ok(Self {
            client,
            cached_status: status,
            last_update: Instant::now(),
            exit: false,
            show_hint: false,
            app_mode: AppMode::Normal,
            task_input: TaskInput::default(),
        })
    }

    /// Main event loop (now async)
    pub async fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> io::Result<()> {
        while !self.exit {
            // Update cached status periodically
            if self.last_update.elapsed() > Duration::from_millis(100) {
                if let Ok(status) = self.client.get_status().await {
                    self.cached_status = status;
                    self.last_update = Instant::now();
                }
            }

            // Render UI using cached status
            terminal.draw(|f| self.draw(f))?;

            // Handle events with timeout for continuous updates
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key_event) = event::read()? {
                    self.handle_key_event(key_event).await?;
                }
            }
        }
        Ok(())
    }

    /// Handle key events (now async for network calls)
    async fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
        match self.app_mode {
            AppMode::Normal => {
                if let Some(command) = KeyCommand::from_keycode(key_event.code) {
                    self.execute_command(command).await?;
                }
            }
            AppMode::Input => {
                // Input handling remains synchronous
                match key_event.code {
                    KeyCode::Enter => {
                        let task_name = self.task_input.confirm_task();
                        self.client.send_request(Request::SetTask(task_name)).await?;
                        self.app_mode = AppMode::Normal;
                    }
                    // ... other input handling
                }
            }
        }
        Ok(())
    }

    /// Execute commands via network (async)
    async fn execute_command(&mut self, command: KeyCommand) -> Result<(), Box<dyn std::error::Error>> {
        let request = match command {
            KeyCommand::Quit => {
                self.exit = true;
                return Ok(());
            }
            KeyCommand::ToggleHint => {
                self.show_hint = !self.show_hint;
                return Ok(());
            }
            KeyCommand::InputTask => {
                self.app_mode = self.app_mode.toggle();
                return Ok(());
            }
            KeyCommand::Reset => Request::Reset,
            KeyCommand::Toggle => Request::Toggle,
            KeyCommand::SwitchMode => Request::SwitchMode,
            KeyCommand::SetLong => Request::SetPreset("long".to_string()),
            KeyCommand::SetShort => Request::SetPreset("short".to_string()),
            KeyCommand::SetTest => Request::SetPreset("test".to_string()),
        };

        // Send request and update cached status
        self.client.send_request(request).await?;
        if let Ok(status) = self.client.get_status().await {
            self.cached_status = status;
        }
        
        Ok(())
    }

    /// Render UI using cached status (synchronous)
    fn draw(&self, f: &mut Frame) {
        // Use self.cached_status instead of self.timer
        // Rest of rendering logic remains the same
    }
}
```

### Client Implementation (`src/client/network.rs`)
```rust
// Add to App struct (future enhancement)
pub struct App {
    timer: Timer,
    exit: bool,
    app_mode: AppMode,
    task_input: TaskInput,
    show_hint: bool,
    // client: Option<BridgeClient>, // Add this field later
}

// Future enhancement: detect server and connect automatically
impl App {
    pub fn new() -> Self {
        // TODO: Try to connect to server, fall back to standalone
        Self {
            timer: Timer::new(),
            exit: false,
            app_mode: AppMode::default(),
            task_input: TaskInput::new(),
            show_hint: false,
        }
    }
}
```

## Testing Strategy

### Manual Testing with netcat
```bash
# Start server
cargo run -- --server

# Test basic commands (in another terminal)
echo '{"GetStatus":null}' | nc 127.0.0.1 8080
echo '{"SetTask":"Write tests"}' | nc 127.0.0.1 8080  
echo '{"Start":null}' | nc 127.0.0.1 8080
echo '{"GetStatus":null}' | nc 127.0.0.1 8080
echo '{"Pause":null}' | nc 127.0.0.1 8080
echo '{"Reset":null}' | nc 127.0.0.1 8080
echo '{"SwitchMode":null}' | nc 127.0.0.1 8080
echo '{"SetPreset":"long"}' | nc 127.0.0.1 8080
echo '{"Ping":null}' | nc 127.0.0.1 8080
```

### Basic Integration Tests
```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_basic_server_functionality() {
    // Start server in background
    // Connect client
    // Send requests
    // Verify responses
}
```

## Deployment

### Local Development
```bash
# Build and run
cargo build --release
./target/release/pomo-bridge --server

# Or during development
cargo run -- --server
```

### Systemd Service (Optional)
```ini
# /etc/systemd/system/pomo-bridge.service
[Unit]
Description=PomoBridge TCP Server
After=network.target

[Service]
Type=simple
User=pomo
ExecStart=/usr/local/bin/pomo-bridge --server
Restart=always

[Install]
WantedBy=multi-user.target
```

## Future Evolution Path

### Phase 1: Basic TCP Bridge (1-2 days) ✅
- Single shared timer
- Basic request/response
- Manual testing with netcat

### Phase 2: TUI Client Mode (1 week)
- Modify TUI to connect to server
- Fallback to standalone mode
- Basic error handling

### Phase 3: Simple CLI Client (1 week)
- `pomo-cli status`
- `pomo-cli start --task "coding"`
- `pomo-cli pause/resume/reset`

### Phase 4: Enhanced Features (2-4 weeks)
- Configuration files
- Better error handling
- Authentication (optional)
- Real-time updates

### Phase 5: Production Features (4+ weeks)
- Multiple client types
- Event broadcasting
- Advanced configuration
- Monitoring and logging

## Success Metrics

### Phase 1 Goals
1. ✅ Server accepts TCP connections
2. ✅ Basic timer control via TCP
3. ✅ Multiple clients can connect
4. ✅ Existing TUI works unchanged
5. ✅ Can test with standard tools (netcat)

### Performance Targets
- **Latency**: <10ms for local connections
- **Throughput**: Handle 10+ concurrent clients
- **Memory**: <5MB additional memory usage
- **CPU**: Minimal impact on existing performance

## Step-by-Step Implementation Plan

### **Strategy: Incremental Development with Continuous Testing**

This plan allows you to implement the Smart Local Server architecture **without breaking your existing system** at any point. Each step is testable and can be rolled back if needed.

---

## **Phase 1: Foundation Setup (Day 1)**
*Goal: Set up project structure and basic protocol without touching existing code*

### **Step 1.1: Backup and Branch (15 minutes)**
```bash
# Create implementation branch
git checkout -b feature/smart-server
git add -A && git commit -m "Backup: Working monolithic version"

# Your existing `cargo run` still works at this point
```

### **Step 1.2: Add Dependencies (15 minutes)**
```bash
# Add to Cargo.toml (existing dependencies stay unchanged)
[dependencies]
# ... existing deps ...
tokio = { version = "1.0", features = ["full"] }
hyper = { version = "1.0", features = ["full"] }
hyper-util = "0.1"
http-body-util = "0.1"

# Test: Existing functionality still works
cargo build
cargo run  # Should work exactly as before
```

### **Step 1.3: Create Module Structure (30 minutes)**
```bash
# Create new directories (don't modify existing files yet)
mkdir -p src/protocol src/server src/client

# Create empty module files
touch src/protocol/mod.rs src/protocol/messages.rs src/protocol/http.rs
touch src/server/mod.rs src/server/core.rs src/server/tcp.rs src/server/http.rs  
touch src/client/mod.rs src/client/tcp.rs src/client/cache.rs

# Test: Still compiles and runs
cargo build
cargo run  # Existing TUI works perfectly
```

### **Step 1.4: Implement Protocol Messages (1 hour)**
```rust
// src/protocol/mod.rs
pub mod messages;
pub mod http;
pub use messages::{Request, Response, TimerStatus};

// src/protocol/messages.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    GetStatus,
    Start,
    Pause,
    Resume,
    Reset,
    SwitchMode,
    SetTask(String),
    SetPreset(String), // "short", "long", "test"
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Ok,
    Status(TimerStatus),
    Error(String),
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerStatus {
    pub mode: String,
    pub remaining_secs: u64,
    pub is_running: bool,
    pub is_paused: bool,
    pub is_idle: bool,
    pub task: String,
    pub preset: String,
}

// Test: Protocol compiles
cargo build
cargo run  # Original TUI unchanged
```

---

## **Phase 2: Server Implementation (Day 2)**
*Goal: Create working server that can be tested independently*

### **Step 2.1: Async Server Core (2 hours)**
```rust
// src/server/mod.rs
pub mod core;
pub mod tcp;
pub mod http;
pub use core::PomoServer;

// src/server/core.rs
use tokio::sync::Mutex;  // Async mutex for better concurrency
use std::sync::Arc;
use crate::{timer::Timer, protocol::{Request, Response, TimerStatus}};

#[derive(Clone)]  // Make cloneable for sharing across async tasks
pub struct PomoServer {
    timer: Arc<Mutex<Timer>>,  // Tokio async mutex
}

impl PomoServer {
    pub fn new() -> Self {
        Self {
            timer: Arc::new(Mutex::new(Timer::new())),
        }
    }

    /// Process client requests asynchronously
    pub async fn process_request(&self, request: Request) -> Response {
        // Async lock - yields to other tasks while waiting
        let mut timer = self.timer.lock().await;

        match request {
            Request::GetStatus => Response::Status(self.get_status(&timer)),
            Request::Start => {
                if timer.is_idle() || timer.is_paused() {
                    timer.toggle();
                }
                Response::Ok
            }
            Request::Pause => {
                if timer.is_running() && !timer.is_paused() {
                    timer.toggle();
                }
                Response::Ok
            }
            Request::Resume => {
                if timer.is_paused() {
                    timer.toggle();
                }
                Response::Ok
            }
            Request::Reset => {
                timer.reset();
                Response::Ok
            }
            Request::SwitchMode => {
                timer.switch_mode();
                Response::Ok
            }
            Request::SetTask(name) => {
                timer.set_task_name(&name);
                Response::Ok
            }
            Request::SetPreset(preset) => {
                use crate::timer::Preset;
                let preset_enum = match preset.as_str() {
                    "short" => Preset::Short,
                    "long" => Preset::Long,
                    "test" => Preset::Test,
                    _ => return Response::Error(format!("Unknown preset: {}", preset)),
                };
                timer.set_preset(preset_enum);
                Response::Ok
            }
            Request::Ping => Response::Pong,
        }
    }

    fn get_status(&self, timer: &Timer) -> TimerStatus {
        TimerStatus {
            mode: timer.get_mode().to_string(),
            remaining_secs: timer.get_remaining().as_secs(),
            is_running: timer.is_running(),
            is_paused: timer.is_paused(),
            is_idle: timer.is_idle(),
            task: timer.get_task_name().to_string(),
            preset: format!("{:?}", timer.get_preset()).to_lowercase(),
        }
    }
}

// Test: Async server logic compiles
cargo build
cargo run  # Original TUI still works
```

### **Step 2.2: Add Missing Timer Methods (30 minutes)**
```rust
// Add to src/timer.rs (minimal changes)
impl Timer {
    // Add this method if missing
    pub fn get_preset(&self) -> Preset {
        self.timeset
    }
}

// Test: Timer enhancements work
cargo build
cargo run  # Original TUI + new methods work
```

### **Step 2.3: Async TCP Server Implementation (1.5 hours)**
```rust
// src/server/tcp.rs
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use crate::{server::PomoServer, protocol::{Request, Response}};

impl PomoServer {
    pub async fn start_tcp_server(&self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        println!("🔌 TCP server listening on {} (TUI clients)", addr);

        while let Ok((stream, client_addr)) = listener.accept().await {
            println!("📱 TUI client connected: {}", client_addr);
            let server = self.clone(); // PomoServer is now cloneable
            
            tokio::spawn(async move {
                if let Err(e) = handle_tcp_client(stream, server).await {
                    eprintln!("❌ TCP client error: {}", e);
                }
            });
        }
        Ok(())
    }
}

async fn handle_tcp_client(
    stream: TcpStream,
    server: PomoServer,  // No Arc needed since PomoServer is cloneable
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await? {
            0 => {
                println!("📱 TUI client disconnected");
                break;
            }
            _ => {
                let request: Request = match serde_json::from_str(line.trim()) {
                    Ok(req) => req,
                    Err(e) => {
                        let error_response = Response::Error(format!("Invalid JSON: {}", e));
                        send_response(&stream, error_response).await?;
                        continue;
                    }
                };

                // Async request processing - non-blocking!
                let response = server.process_request(request).await;
                send_response(&stream, response).await?;
            }
        }
    }
    Ok(())
}

async fn send_response(
    stream: &TcpStream,
    response: Response,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(&response)?;
    let mut writer = stream;
    writer.write_all((json + "\n").as_bytes()).await?;
    Ok(())
}

// Test: Async TCP server compiles
cargo build
cargo run  # Original TUI works
```

### **Step 2.4: Test Async TCP Server (30 minutes)**
```rust
// Add temporary test binary: src/bin/test-server.rs
use pomo_server::server::PomoServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing async TCP server...");
    let server = PomoServer::new();
    server.start_tcp_server("127.0.0.1:8080").await
}
```

```bash
# Test async TCP server independently
cargo run --bin test-server

# In another terminal, test with netcat
echo '{"GetStatus":null}' | nc 127.0.0.1 8080
echo '{"Start":null}' | nc 127.0.0.1 8080
echo '{"GetStatus":null}' | nc 127.0.0.1 8080

# Test concurrent clients (should not block each other)
echo '{"Ping":null}' | nc 127.0.0.1 8080 &
echo '{"GetStatus":null}' | nc 127.0.0.1 8080 &
echo '{"Start":null}' | nc 127.0.0.1 8080 &

# Original TUI still works in third terminal
cargo run
```

**🎯 Async Benefits Verification:**
- Multiple netcat connections should work simultaneously
- No client should block others during processing
- Server should handle concurrent requests efficiently

---

## **Phase 3: HTTP Server (Day 3)**
*Goal: Add HTTP API for scripts while keeping TCP working*

### **Step 3.1: Async HTTP Server Implementation (2 hours)**
```rust
// src/server/http.rs
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming as IncomingBody, Request as HyperRequest, Response as HyperResponse};
use hyper::{Method, StatusCode};
use http_body_util::Full;
use hyper::body::Bytes;
use tokio::net::TcpListener;
use crate::protocol::Request;

impl PomoServer {
    pub async fn start_http_server(&self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        println!("🌐 HTTP server listening on {} (scripts/APIs)", addr);

        while let Ok((stream, _)) = listener.accept().await {
            let server = self.clone();
            
            tokio::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(stream, service_fn(move |req| {
                        let server = server.clone();
                        async move { handle_http_request(req, server).await }
                    }))
                    .await
                {
                    eprintln!("❌ HTTP connection error: {}", err);
                }
            });
        }
        Ok(())
    }
}

async fn handle_http_request(
    req: HyperRequest<IncomingBody>,
    server: PomoServer,
) -> Result<HyperResponse<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    let path = req.uri().path();
    let method = req.method();

    let (status, body) = match (method, path) {
        (&Method::GET, "/status") => {
            // Async request processing
            let response = server.process_request(Request::GetStatus).await;
            (StatusCode::OK, serde_json::to_string(&response)?)
        }
        (&Method::POST, "/start") => {
            server.process_request(Request::Start).await;
            (StatusCode::OK, "Started".to_string())
        }
        (&Method::POST, "/pause") => {
            server.process_request(Request::Pause).await;
            (StatusCode::OK, "Paused".to_string())
        }
        (&Method::POST, "/resume") => {
            server.process_request(Request::Resume).await;
            (StatusCode::OK, "Resumed".to_string())
        }
        (&Method::POST, "/reset") => {
            server.process_request(Request::Reset).await;
            (StatusCode::OK, "Reset".to_string())
        }
        (&Method::POST, "/switch") => {
            server.process_request(Request::SwitchMode).await;
            (StatusCode::OK, "Switched mode".to_string())
        }
        (&Method::GET, "/ping") => {
            server.process_request(Request::Ping).await;
            (StatusCode::OK, "Pong".to_string())
        }
        _ => (StatusCode::NOT_FOUND, "Not Found".to_string()),
    };

    let response = HyperResponse::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")  // CORS for web clients
        .body(Full::new(Bytes::from(body)))?;

    Ok(response)
}
```

### **Step 3.2: Test Dual Protocol Server (30 minutes)**
```rust
// Update src/bin/test-server.rs
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = PomoServer::new();
    
    // Start both servers concurrently
    let tcp_server = server.start_tcp_server("127.0.0.1:8080");
    let http_server = server.start_http_server("127.0.0.1:8081");
    
    tokio::try_join!(tcp_server, http_server)?;
    Ok(())
}
```

```bash
# Test both protocols
cargo run --bin test-server

# Terminal 2: Test TCP
echo '{"GetStatus":null}' | nc 127.0.0.1 8080

# Terminal 3: Test HTTP  
curl http://127.0.0.1:8081/status
curl -X POST http://127.0.0.1:8081/start
curl http://127.0.0.1:8081/status

# Terminal 4: Original TUI still works
cargo run
```

---

## **Phase 4: Client Implementation (Day 4)**
*Goal: Create network client without breaking existing TUI*

### **Step 4.1: Async TCP Client (1.5 hours)**
```rust
// src/client/mod.rs
pub mod tcp;
pub mod cache;
pub use tcp::PomoClient;

// src/client/tcp.rs
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::time::{timeout, Duration};
use crate::protocol::{Request, Response, TimerStatus};

pub struct PomoClient {
    reader: Option<BufReader<TcpStream>>,
    writer: Option<BufWriter<TcpStream>>,
    addr: String,
}

impl PomoClient {
    pub fn new(addr: String) -> Self {
        Self { 
            reader: None,
            writer: None,
            addr 
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let stream = TcpStream::connect(&self.addr).await?;
        // Clone stream for reader and writer
        let reader_stream = stream.try_clone().await?;
        
        self.reader = Some(BufReader::new(reader_stream));
        self.writer = Some(BufWriter::new(stream));
        
        Ok(())
    }

    pub async fn send_request(&mut self, request: Request) -> Result<Response, Box<dyn std::error::Error>> {
        let writer = self.writer.as_mut().ok_or("Not connected")?;
        let reader = self.reader.as_mut().ok_or("Not connected")?;
        
        // Send request with timeout
        let json = serde_json::to_string(&request)?;
        timeout(Duration::from_secs(5), async {
            writer.write_all((json + "\n").as_bytes()).await?;
            writer.flush().await
        }).await??;
        
        // Read response with timeout
        let mut line = String::new();
        timeout(Duration::from_secs(5), reader.read_line(&mut line)).await??;
        
        let response: Response = serde_json::from_str(line.trim())?;
        Ok(response)
    }

    pub async fn get_status(&mut self) -> Result<TimerStatus, Box<dyn std::error::Error>> {
        match self.send_request(Request::GetStatus).await? {
            Response::Status(status) => Ok(status),
            Response::Error(e) => Err(e.into()),
            _ => Err("Unexpected response type".into()),
        }
    }

    /// Convenience methods for common operations
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_request(Request::Start).await?;
        Ok(())
    }

    pub async fn pause(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_request(Request::Pause).await?;
        Ok(())
    }

    pub async fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_request(Request::Reset).await?;
        Ok(())
    }

    pub async fn ping(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        match self.send_request(Request::Ping).await? {
            Response::Pong => Ok(true),
            _ => Ok(false),
        }
    }
}
```

### **Step 4.2: Test Client Independently (30 minutes)**
```rust
// Add test binary: src/bin/test-client.rs
use pomo_server::client::PomoClient;
use pomo_server::protocol::Request;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = PomoClient::new("127.0.0.1:8080".to_string());
    client.connect().await?;
    
    println!("Testing client...");
    
    let status = client.get_status().await?;
    println!("Status: {:?}", status);
    
    client.send_request(Request::Start).await?;
    println!("Started timer");
    
    let status = client.get_status().await?;
    println!("Status after start: {:?}", status);
    
    Ok(())
}
```

```bash
# Test client
cargo run --bin test-server  # Terminal 1
cargo run --bin test-client  # Terminal 2
cargo run                    # Terminal 3: Original TUI works
```

---

## **Phase 5: Smart Main Entry (Day 5)**
*Goal: Add server detection and auto-spawning while preserving fallback*

### **Step 5.1: Server Detection Logic (1 hour)**
```rust
// Update src/main.rs (keep original as fallback)
mod timer;
mod tui;
mod utils;
mod logging;
mod protocol;
mod server;
mod client;

use std::env;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};

const TCP_ADDR: &str = "127.0.0.1:8080";
const HTTP_ADDR: &str = "127.0.0.1:8081";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    match args.get(1).map(|s| s.as_str()) {
        Some("--server") => {
            // Pure server mode
            println!("🚀 Starting Pomo server (daemon mode)...");
            start_server().await
        }
        Some("--fallback") => {
            // Force fallback mode (original TUI)
            println!("🔄 Starting in fallback mode...");
            start_original_tui().await
        }
        Some("--help") => {
            print_help();
            Ok(())
        }
        _ => {
            // Smart mode: try network, fallback to original
            if server_exists().await {
                println!("📡 Connecting to existing server...");
                start_network_tui().await
            } else {
                println!("🚀 Starting embedded server + TUI...");
                start_embedded_server_and_tui().await
            }
        }
    }
}

async fn server_exists() -> bool {
    TcpStream::connect(TCP_ADDR).await.is_ok()
}

async fn start_server() -> Result<(), Box<dyn std::error::Error>> {
    let server = server::PomoServer::new();
    let tcp_server = server.start_tcp_server(TCP_ADDR);
    let http_server = server.start_http_server(HTTP_ADDR);
    tokio::try_join!(tcp_server, http_server)?;
    Ok(())
}

async fn start_original_tui() -> Result<(), Box<dyn std::error::Error>> {
    // Keep original TUI code as fallback
    let mut terminal = ratatui::init();
    let app_result = tui::App::new().run(&mut terminal);
    ratatui::restore();
    app_result.map_err(|e| e.into())
}

async fn start_network_tui() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement in Phase 6
    println!("Network TUI not implemented yet, falling back...");
    start_original_tui().await
}

async fn start_embedded_server_and_tui() -> Result<(), Box<dyn std::error::Error>> {
    // Spawn server in background
    let server = server::PomoServer::new();
    tokio::spawn(async move {
        let tcp_server = server.start_tcp_server(TCP_ADDR);
        let http_server = server.start_http_server(HTTP_ADDR);
        if let Err(e) = tokio::try_join!(tcp_server, http_server) {
            eprintln!("Server error: {}", e);
        }
    });
    
    // Wait for server to start
    for _ in 0..20 {
        sleep(Duration::from_millis(100)).await;
        if server_exists().await {
            break;
        }
    }
    
    if server_exists().await {
        start_network_tui().await
    } else {
        println!("⚠️  Server failed to start, using fallback mode");
        start_original_tui().await
    }
}

fn print_help() {
    println!("PomoServer - Smart Local Pomodoro Timer");
    println!();
    println!("USAGE:");
    println!("    pomo                     # Smart mode (auto-detect/spawn server)");
    println!("    pomo --server            # Server only (daemon mode)");
    println!("    pomo --fallback          # Original TUI (no networking)");
    println!("    pomo --help              # Show this help");
    println!();
    println!("TESTING:");
    println!("    # TCP API");
    println!("    echo '{{\"GetStatus\":null}}' | nc 127.0.0.1 8080");
    println!("    # HTTP API");
    println!("    curl http://127.0.0.1:8081/status");
}
```

### **Step 5.2: Test Smart Entry Point (30 minutes)**
```bash
# Test all modes work
cargo run --help                    # Help works
cargo run --fallback               # Original TUI works  
cargo run --server                 # Server starts
cargo run                          # Auto-spawns server + TUI (fallback for now)

# Test server detection
cargo run --server &                # Start server in background
cargo run                          # Should detect existing server
```

---

## **Phase 6: Network TUI (Day 6-7)**
*Goal: Replace TUI timer with network client, keeping UI identical*

### **Step 6.1: Create Network-Based TUI (3-4 hours)**
```rust
// Create src/tui_network.rs (don't modify existing tui.rs yet)
use crate::client::PomoClient;
use crate::protocol::{Request, TimerStatus};
use crate::utils::{self, centered_area, create_large_ascii_numbers, render_hint, KeyCommand};
// ... other imports same as original

pub struct NetworkApp {
    client: PomoClient,
    cached_status: TimerStatus,
    last_update: std::time::Instant,
    
    // Keep all existing UI state
    exit: bool,
    show_hint: bool,
    app_mode: AppMode,
    task_input: TaskInput,
}

impl NetworkApp {
    pub async fn new(server_addr: String) -> Result<Self, Box<dyn std::error::Error>> {
        let mut client = PomoClient::new(server_addr);
        client.connect().await?;
        
        let cached_status = client.get_status().await?;
        
        Ok(Self {
            client,
            cached_status,
            last_update: std::time::Instant::now(),
            exit: false,
            show_hint: false,
            app_mode: AppMode::Normal,
            task_input: TaskInput::default(),
        })
    }

    pub async fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> std::io::Result<()> {
        while !self.exit {
            // Update status periodically
            if self.last_update.elapsed() > Duration::from_millis(200) {
                if let Ok(status) = self.client.get_status().await {
                    self.cached_status = status;
                    self.last_update = std::time::Instant::now();
                }
            }

            // Render using cached status (same as original)
            terminal.draw(|f| self.draw(f))?;

            // Handle events (same polling as original)
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key_event) = event::read()? {
                    if let Err(e) = self.handle_key_event(key_event).await {
                        eprintln!("Network error: {}, continuing...", e);
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<(), Box<dyn std::error::Error>> {
        match self.app_mode {
            AppMode::Normal => {
                if let Some(command) = KeyCommand::from_keycode(key_event.code) {
                    self.execute_command(command).await?;
                }
            }
            AppMode::Input => {
                // Input handling stays synchronous (same as original)
                match key_event.code {
                    KeyCode::Enter => {
                        let task_name = self.task_input.confirm_task();
                        self.client.send_request(Request::SetTask(task_name)).await?;
                        self.app_mode = AppMode::Normal;
                    }
                    // ... rest same as original
                }
            }
        }
        Ok(())
    }

    async fn execute_command(&mut self, command: KeyCommand) -> Result<(), Box<dyn std::error::Error>> {
        let request = match command {
            KeyCommand::Quit => {
                self.exit = true;
                return Ok(());
            }
            KeyCommand::ToggleHint => {
                self.show_hint = !self.show_hint;
                return Ok(());
            }
            KeyCommand::InputTask => {
                self.app_mode = self.app_mode.toggle();
                return Ok(());
            }
            KeyCommand::Reset => Request::Reset,
            KeyCommand::Toggle => {
                if self.cached_status.is_paused {
                    Request::Resume
                } else if self.cached_status.is_running {
                    Request::Pause
                } else {
                    Request::Start
                }
            }
            KeyCommand::SwitchMode => Request::SwitchMode,
            KeyCommand::SetLong => Request::SetPreset("long".to_string()),
            KeyCommand::SetShort => Request::SetPreset("short".to_string()),
            KeyCommand::SetTest => Request::SetPreset("test".to_string()),
        };

        self.client.send_request(request).await?;
        
        // Immediately update cached status for responsiveness
        if let Ok(status) = self.client.get_status().await {
            self.cached_status = status;
        }
        
        Ok(())
    }

    fn draw(&self, f: &mut Frame) {
        // Use self.cached_status instead of self.timer
        // Rest of drawing logic identical to original
        // ... copy from original tui.rs but use cached_status
    }
}
```

### **Step 6.2: Integrate Network TUI (1 hour)**
```rust
// Update src/main.rs to use network TUI
async fn start_network_tui() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = ratatui::init();
    let mut app = tui::NetworkApp::new(TCP_ADDR.to_string()).await?;
    let app_result = app.run(&mut terminal).await;
    ratatui::restore();
    app_result.map_err(|e| e.into())
}
```

### **Step 6.3: Test Network TUI (30 minutes)**
```bash
# Test network TUI
cargo run --server &                # Start server
cargo run                          # Should use network TUI
# Verify: All keys work, UI looks identical, timer updates

# Test fallback still works
pkill -f "pomo.*server"            # Kill server
cargo run --fallback              # Original TUI works
```

---

## **Phase 7: Integration Testing (Day 8)**
*Goal: Test all scenarios and edge cases*

### **Step 7.1: Multi-Client Testing (1 hour)**
```bash
# Terminal 1: Start server
cargo run --server

# Terminal 2: First TUI client
cargo run

# Terminal 3: Second TUI client  
cargo run

# Terminal 4: HTTP API testing
curl http://127.0.0.1:8081/status
curl -X POST http://127.0.0.1:8081/start
curl http://127.0.0.1:8081/status

# Verify: All clients show same timer state
```

### **Step 7.2: Error Handling Testing (1 hour)**
```bash
# Test server crash recovery
cargo run --server &
cargo run                          # Start TUI
pkill -f "pomo.*server"            # Kill server
# Verify: TUI handles gracefully

# Test network issues
cargo run                          # Auto-spawn mode
# Verify: Server spawns, TUI connects

# Test fallback mode
cargo run --fallback
# Verify: Works without networking
```

### **Step 7.3: Performance Testing (30 minutes)**
```bash
# Test responsiveness
cargo run --server &
cargo run
# Verify: UI updates smoothly, key presses responsive

# Test resource usage
htop  # Monitor CPU/memory while running
```

---

## **Phase 8: Documentation and Cleanup (Day 9)**
*Goal: Polish and document the system*

### **Step 8.1: Clean Up Test Binaries (30 minutes)**
```bash
# Remove test binaries
rm src/bin/test-server.rs src/bin/test-client.rs

# Update Cargo.toml if needed
# Clean up any debug prints
```

### **Step 8.2: Add Integration Examples (1 hour)**
```bash
# Create examples/ directory
mkdir examples

# Add Waybar integration example
cat > examples/waybar-config.json << 'EOF'
{
    "custom/pomo": {
        "exec": "curl -s http://127.0.0.1:8081/status | jq -r '.Status.mode + \" \" + (.Status.remaining_secs/60|floor|tostring) + \"m\"'",
        "interval": 5,
        "format": "🍅 {}"
    }
}
EOF

# Add shell script examples
cat > examples/pomo-control.sh << 'EOF'
#!/bin/bash
case "$1" in
    start) curl -X POST http://127.0.0.1:8081/start ;;
    pause) curl -X POST http://127.0.0.1:8081/pause ;;
    status) curl -s http://127.0.0.1:8081/status | jq ;;
    *) echo "Usage: $0 {start|pause|status}" ;;
esac
EOF
```

### **Step 8.3: Final Testing (1 hour)**
```bash
# Complete system test
cargo build --release

# Test all modes
./target/release/pomo --help
./target/release/pomo --server &
./target/release/pomo --client
./target/release/pomo --fallback
./target/release/pomo  # Auto mode

# Test examples
chmod +x examples/pomo-control.sh
./examples/pomo-control.sh status
```

---

## **Rollback Strategy**

At any point, you can rollback to working state:

```bash
# Rollback to previous working version
git stash  # Save current work
git checkout main  # or your previous branch
cargo run  # Original version works

# Or continue from last working step
git checkout feature/smart-server
git reset --hard HEAD~1  # Go back one commit
```

## **Success Criteria**

✅ **Original functionality preserved**: `cargo run --fallback` works exactly as before  
✅ **Smart mode works**: `cargo run` detects/spawns server automatically  
✅ **Multi-client support**: Multiple TUI instances share timer state  
✅ **API access**: HTTP endpoints work for scripts  
✅ **Graceful fallbacks**: Network failures don't break functionality  
✅ **Performance**: UI remains responsive with network calls  

## **Total Timeline: 8-9 days**

This incremental approach ensures you always have a working system while building toward the Smart Local Server architecture. Each phase can be tested independently and rolled back if needed.

**Key Advantage**: Your existing workflow is never broken - you can always fall back to the original monolithic TUI while developing the networked version!

---

## **Architecture Summary**

### **🚀 Key Improvements in This Design**

#### **1. Async-First Architecture**
- **Tokio async mutex** instead of std sync mutex
- **Non-blocking request processing** - multiple clients don't block each other
- **Built-in timeouts** and error handling for network operations
- **Future-proof** for advanced async features

#### **2. Dual Protocol Excellence**
```
TCP:8080  ←→  [Async Server Core]  ←→  HTTP:8081
   ↑              ↑                      ↑
TUI Clients   Shared Timer           Scripts/APIs
(persistent)   (async mutex)         (stateless)
```

#### **3. Smart Startup Logic**
```bash
pomo                # Auto-detect → spawn server if needed → connect TUI
pomo --server      # Pure daemon mode (no TUI)
pomo --fallback    # Original monolithic TUI (always works)
```

#### **4. Incremental Safety**
- **Never breaks existing functionality** during development
- **Phase-by-phase testing** with rollback at each step
- **Continuous validation** that original TUI works

### **🎯 Performance Benefits**

| Aspect | Sync Approach | Async Approach |
|--------|---------------|----------------|
| **Concurrency** | Clients block each other | True concurrent processing |
| **Resource Usage** | Thread per client | Task-based (efficient) |
| **Scalability** | Limited by threads | Scales to thousands |
| **Responsiveness** | Degrades under load | Maintains performance |
| **Error Handling** | Basic | Timeouts + retries |

### **📡 Protocol Comparison**

| Use Case | Protocol | Why |
|----------|----------|-----|
| **TUI Clients** | TCP | Persistent connection, real-time updates, low latency |
| **Scripts/Waybar** | HTTP | Stateless, standard tooling, easy testing |
| **Future Web UI** | HTTP | Browser compatibility, REST conventions |
| **Mobile Apps** | HTTP | Standard HTTP clients, JSON APIs |

### **🔧 Implementation Highlights**

#### **Server Core**
```rust
// Async, cloneable, efficient
#[derive(Clone)]
pub struct PomoServer {
    timer: Arc<Mutex<Timer>>,  // Tokio async mutex
}

// Non-blocking request processing
pub async fn process_request(&self, request: Request) -> Response {
    let timer = self.timer.lock().await;  // Yields to other tasks
    // ... process request
}
```

#### **Client Benefits**
```rust
// Built-in timeouts and error handling
timeout(Duration::from_secs(5), async {
    client.send_request(Request::GetStatus).await
}).await??;
```

#### **Testing Strategy**
```bash
# Concurrent client testing
echo '{"Ping":null}' | nc 127.0.0.1 8080 &
echo '{"GetStatus":null}' | nc 127.0.0.1 8080 &
echo '{"Start":null}' | nc 127.0.0.1 8080 &
# All should complete without blocking each other
```

This async architecture provides **enterprise-grade concurrency** while maintaining the **simplicity and safety** needed for personal productivity use!
