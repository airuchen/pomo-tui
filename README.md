# POMO-TUI

A Rust-based Pomodoro TUI built on top of [Ratatui](https://github.com/ratatui/ratatui).

<table>
<tr>
<td><img src="./resources/work.png" width="260"/></td>
<td><img src="./resources/break.png" width="260"/></td>
<td><img src="./resources/keymap.png" width="260"/></td>
</tr>
</table>

### Design

`pomo-tui` follows a **client–server architecture** with both **TCP** and **HTTP** interfaces.

* **TCP** is used for TUI clients — low overhead, persistent connection.
* **HTTP** exposes a REST API for anything that might want to interact with the timer (Waybar, scripts, web UI, mobile app… contributions welcome).

```
                         ┌──────────────────────────────────┐
                         │            Pomo Server           │
                         │                                  │
                         │   ┌──────────────────────────┐   │
                         │   │        Timer Logic       │   │
                         │   └──────────────────────────┘   │
                         │                                  │
                         │   ┌──────────────┐ ┌──────────┐  │
                         │   │  TCP Server  │ │ HTTP API │  │
                         │   │ (TUI Proto)  │ │ (REST)   │  │
                         │   └──────────────┘ └──────────┘  │
                         │              │                   │
                         └──────────────┼───────────────────┘
                 ┌──────────────────────┼
                 │                      │
    ┌─────────────────────┐   ┌─────────────────────┐   ┌─────────────────────────────┐
    │    TUI Client 1     │   │     TUI Client 2    │   │    Other Clients,           │
    │                     │   │                     │   │    Future Integrations,     │
    │  (TCP connection)   │   │  (TCP connection)   │   │ (REST: Waybar, Scripts,     │
    │                     │   │                     │   │  Web UI, Mobile App …)      │
    └─────────────────────┘   └─────────────────────┘   └─────────────────────────────┘
```

### Installation

##### Using Cargo

```bash
git clone https://github.com/airuchen/pomo-tui.git
cd pomo-tui

# Run directly
cargo run

# Or install globally
cargo install --path .
```

##### Release Binary
1. Download from the [Releases page](https://github.com/airuchen/pomo-tui)
2. Extract
    ```bash
    tar -xzvf pomo-tui-<version>-x86_64-linux-musl.tar.gz
    ```
3. Run
    ```bash
    pomo-tui
    ```

### Usage

`pomo-tui` can run as a **server**, a **client**, or both at once.

If you just run `pomo-tui`, it will:

1. Check if a server is already running.
2. If yes → run in **client mode** and connect.
3. If not → spawn a **server + client** (embedded mode).

This way, multiple terminals can share the same timer state without needing to think about it too much.

### Modes

* **Server Mode**
Runs only the server, no UI.

```
pomo-tui --server
```

* **Embedded Mode (default if no server detected)**
Runs both the server and the TUI.

```
pomo-tui
```

* **Client Mode (if server is detected)**
Only runs the TUI and connects to the existing server.

```
pomo-tui
```

>  You can override the default `127.0.0.1:1880` / `127.0.0.1:1881` using:
>
> ```
> pomo-tui --server --tcp-addr 127.0.0.1:1880 --http-addr 127.0.0.1:1881
> ```

### Back Story

I couldn’t find a simple Pomodoro TUI on the internet that matched what I had in mind. So I took the chance to practice some Rust and turned it into a small side project.
It’s not perfect — feel free to open issues or, even better, send a PR if something bothers you more than it bothers me.

## TODO

* [ ] Bring logging back from the dead
* [ ] Waybar integration
* [ ] Docs for REST API
