Slack Status
============


Update your Slack status based on your current location.


Features
--------

- [X] Set Slack status depending on your current public IP (either IPv4 or IPv6)
- [X] Customizable address to request public IP address (ip.clara.net by default)
- [X] Option to set status expiration
- [X] Managing saved locations from CLI
- [X] Managing your Slack status from CLI
- [ ] Option to set "on-call" status from PagerDuty
- [ ] Option to use SSID instead of public IP to detect location
- [ ] Option to use OS location APIs?
- [ ] Option to avoid requesting to an Internet server your public IP, like
      using the gateway MAC address to identify the office?

I am only testing `slack-status` on Linux but it is expected to work on most
x86 operating systems.


Getting started
---------------

### Create a Slack App with proper permissions

You must either create a Slack App with `users.profile:read` and
`users.profile:write` rights, and install it manually to your Slack workspace
(you might have to ask a Slack administrator autorization).

When it's done you can get an OAuth Access Token (beginning by `xoxp-...`
followed by random characters), which will be asked during initial setup.

[Legacy Slack tokens](https://api.slack.com/custom-integrations/legacy-tokens)
should work too, but are not recommanded.


### Compilation

You must have a Rust toolchain installed, if you don't have one juste install
[Rustup](https://rustup.rs/) and use default settings, clone this repository,
then run:

```bash
cargo build --release
```


### Installation

You can use `cargo install` or copy the binary (`target/release/slack-status`)
somewhere in your `$PATH`.


### Initial configuration

Run `slack-status` in a terminal, it will launch a wizard if you haven't a
configuration file yet.

By default, configuration file is created at the standard programs configuration
path of your operating system:

* Linux: `/home/<USER>/.config/slack-status/config.toml`
* Mac: `/Users/<USER>/Library/Preferences/com.nsd.slack-status/config.toml`
* Windows: `C:\Users\<USER>\AppData\Roaming\nsd\slack-status\config\config.toml`


### Run `slack-status` automatically

If you want to run `slack-status` automatically you must use the non-interactive
flage:

```bash
slack_status -n
```


#### SystemD services (Linux)

You can find sample SystemD timer and service in the `service` directory. To
use them:

```bash
sudo cp services/linux/* /usr/lib/systemd/user
sudo systemctl daemon-reload
systemctl --user enable slack-status.timer
systemctl --user start slack-status.timer
```


Usage
-----

To add a new location:
```bash
slack-status location add
```

A prompt will appear to help you create a custom status for this location.

To remove one or several locations:
```bash
slack-status location rm
```

A prompt will appear, you can use up and down arrows to navigate through
locations and space bar to select those you want to remove.

To manually set your Slack status:
```bash
slack-status status set
```

A prompt will appear to help you create a status.

Use `slack-status --help` to see every commands available.
