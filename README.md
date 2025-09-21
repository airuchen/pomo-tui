
## Waybar integration

* update `~/.config/waybar/config`

    ```
      "custom/pomodoro": {
        "exec": "cat /home/wen/ws/rust_ws/pomo-tui/pomo_waybar_state.json", // adjust to your file
        "interval": 1,
        "return-type": "json"
      },
    ```
* update `~/.config/waybar/style.css`

    ```css
        #custom-pomodoro {
          color: #89b4fa;              /* Pink color for clock */
          font-weight: bold;
          min-width: 140px;
        }
        #custom-pomodoro.Work   { color: #fab387; }
        #custom-pomodoro.Break  { color: #a6e3a1; }
    ```
