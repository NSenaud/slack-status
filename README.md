Slack Status
============

Update your Slack status automatically depending on your public IP.

Although it works differently, thanks to [slack-loc](https://github.com/kuy/slack-loc)
to save me some time reading documentation. Why using public IP instead of SSID?
Because my company have several offices with the same WiFi SSID, and my goal was
to be able to display in which office I am currently in.


Missing Features
----------------

- [ ] Support for Slack OAuth (currently legacy tokens are used)
- [ ] Option to use SSID instead of public IP to detect location
- [ ] Option to avoid requesting to an Internet server your public IP, like
      using the gateway MAC address to identify the office?


Installation
------------

### Compilation

```bash
cargo build --release
```


### Configuration

An empty configuration file will be created at the standard configuration path
on the first run:

* Linux: `/home/alice/.config/slack-status/config.toml`
* Mac: `/Users/Alice/Library/Preferences/com.nsd.slack-status/config.toml`
* Windows: `C:\Users\Alice\AppData\Roaming\nsd\slack-status\config\config.toml`

You MUST add manually your [legacy Slack token](https://api.slack.com/custom-integrations/legacy-tokens).

### Linux

There is no package available at this time. I am using SystemD but it's up to
you:

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


Development and debugging
-------------------------

### Run with log output

```bash
RUST_LOG=slack_status=debug cargo run
```

You can replace `cargo run` with the binary path.
