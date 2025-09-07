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

#### Task Logging [Optional]
* The pomodoro stores/pipes the current state, allowing waybar to parse the state
* The completed log will sotre in a logging file
    - Session <num> : <task name>
        - start time / break / resume
        - completed time

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
- [x] Main features
    - [x] Basic counter
    - [x] Strat/Stop/Resume
    - [x] State Transition
    - [ ] [Optional] Buffer time (5s) before entering the next state
- [ ] Task Logging
    - [x] Set/Show the current task
    - [ ] Task logging
        - [ ] Log into a file
        - [ ] Log with proper serialization structure
            ```yaml
            Session: <task name>
              - start time / pause / resume
              - break / completed time
            ```
        - [ ] track with uuid so that name doesn't matter.
        - [ ] if the task name change, the logged task name needs to be updated
        - [ ] We should log the current task and the break time before exit the program.
        - [ ] [Optional] statistic mode to visualize chart
    - [ ] Serialization
    - [ ] Pomo state Pipe to waybar
- [ ] TUI
    - [x] ASCII number
    - [ ] Hint
    - [ ] Adjust Font size
- [ ] Clean code
    - [ ] Renaming
    - [ ] Logic cleanup in POMO
        - [ ] Do I need all the member variable to track the state/duration/preset?
- [ ] Sharing
    - [ ] Learning in general
    - [ ] Learning in Rust
