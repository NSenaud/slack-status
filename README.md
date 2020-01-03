Slack Status
============

Update your Slack status automatically depending on your public IP.

Although it doesn't work the same way, thanks to [slack-loc](https://github.com/kuy/slack-loc)
for saving me some time reading documentation. Why using public IP instead of SSID?
Because my company have several offices with the same WiFi SSID, and my goal is
to be able to display in which office I am currently working in.


Features
--------

- [X] Set Slack status depending on your current public IP (v4)
- [X] Status expires after one hour
- [X] Custom address to request public IP address
- [ ] Option to customize status expiration delay
- [ ] Option to set "on-call" status from PagerDuty
- [ ] Managing locations frow CLI
- [ ] Option to use SSID instead of public IP to detect location
- [ ] Option to use OS location APIs?
- [ ] Option to avoid requesting to an Internet server your public IP, like
      using the gateway MAC address to identify the office?


Installation
------------

### Compilation

```bash
cargo build --release
```

You can use `cargo install` or copy the binary somewhere in your `$PATH`.


### Configuration

An empty configuration file will be created at the standard configuration path
if it doesn't exists yet:

* Linux: `/home/alice/.config/slack-status/config.toml`
* Mac: `/Users/Alice/Library/Preferences/com.nsd.slack-status/config.toml`
* Windows: `C:\Users\Alice\AppData\Roaming\nsd\slack-status\config\config.toml`

You must either create a Slack App with `users.profile:write` right, and install
it manually to your workspace to get a OAuth Access Token, or user
[legacy Slack token](https://api.slack.com/custom-integrations/legacy-tokens).
In any case, you have to add the token (`xoxp-...`) to the configuration file.

### Linux

There is no package available at this time. I am using a SystemD timer but it's
up to you:

```bash
sudo cp target/release/slack-status /usr/local/bin
sudo cp services/linux/* /usr/lib/systemd/user
sudo systemctl daemon-reload
systemctl --user enable slack-status.timer
systemctl --user start slack-status.timer
```

### macOS

Should work, LaunchD files to come.

### Windows

It works, must find out how to make a timer on this platform.
