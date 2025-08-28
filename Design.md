# Pomodoro TUI

## Architecture Design
* Layers-Architecture
    - TUI
    - APP
    - Server
* File structure
    - TODO

## Features
#### Pomodoro
* A user can start/pause/resume/restart a pomodoro timer.
* A user can set the current task as the title.
* A user can switch between 25/5 or 50/10 pomodoro.
* A user can check the history to see how many sessions with information of the tasks.
* A user will receive a notification when a session finishes.
* A user can decide if the pomodoro will enter the next session continuously.


#### TUI
* Nice looking text-art.
* The TUI will adjust according to the terminal window size.
* Zen mode to focus.
* Show current time.
* Show hint of how to use the pomodoro.

#### Server [Optional]
* The pomodoro can run as a server in the backgroud.
* A user can request the current status of the pomodoro.
* A user can request to start/pause/resume/restart the timer.

## Technical Stack
* TUI Framework
* Terminal backend: crossterm
* HTTP server: warp
* Async Runtime: tokio
* Nofitication: notify-rust
* CLI: clap
* Serialization: serde + serde_json
* Time: chrono

## TODOs
- [ ] Main features
    - [ ] Basic counter
    - [ ] Strat/Stop/Resume
    - [ ] State Transition
- [ ] TUI
- [ ] Server

