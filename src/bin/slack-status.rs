#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

use std::net::IpAddr;

use clap::App;
use console::{Style, style};
use dialoguer::{theme::ColorfulTheme, Checkboxes, Confirmation, Input, Select};

use slack_status::*;

type Theme = dialoguer::theme::ColorfulTheme;

struct Prompt {
    theme: Theme,
}

fn main() {
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    // Logger setup
    let log_level = match matches.occurrences_of("verbose") {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 | _ => log::LevelFilter::Debug,
    };

    match setup_logger(log_level) {
        Ok(_) => debug!("Logger set up"),
        Err(e) => println!("Failed to setup logger: {}", e),
    };

    let mut first_init = false;

    let prompt = Prompt {
        theme: ColorfulTheme {
            values_style: Style::new().yellow().dim(),
            indicator_style: Style::new().yellow().bold(),
            yes_style: Style::new().yellow().dim(),
            no_style: Style::new().yellow().dim(),
            ..ColorfulTheme::default()
        }
    };

    // Configuration reading, if configuration is not found launch wizard.
    let config = match Config::read(matches.value_of("config")) {
        Ok(c) => match c {
            Some(c) => c,
            None => match configuration_wizard(&prompt, matches.value_of("config")) {
                    Ok(c) => {
                        first_init = true;
                        c
                    },
                    Err(_) => std::process::exit(1),
            },
        },
        Err(e) => {
            error!("Cannot read configuration file: {}", e);
            std::process::exit(1);
        },
    };

    // Init SlackStatus client
    let client = match SlackStatus::from(&config) {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };

    if first_init {
        add_location(&prompt, &client, &config, matches.value_of("config"));
    }

    // Subcommand reading
    if let Some(submatches) = matches.subcommand_matches("location") {
        if let Some(_) = submatches.subcommand_matches("list") {
            // slack-status location list
            list_locations(&client);
        } else if let Some(_) = submatches.subcommand_matches("add") {
            // slack-status location add
            add_location(&prompt, &client, &config, matches.value_of("config"));
        } else if let Some(_) = submatches.subcommand_matches("rm") {
            // slack-status location rm
            rm_location(&prompt, &config, matches.value_of("config"));
        }
    } else if let Some(submatches) = matches.subcommand_matches("status") {
        if let Some(_) = submatches.subcommand_matches("get") {
            // slack-status status get
            get_status(&client);
        } else if let Some(_) = submatches.subcommand_matches("set") {
            // slack-status status set
            set_status(&prompt, &client);
        }
    } else {
        status_update(&client, matches.is_present("noninteractive"));
    }

    std::process::exit(0)
}

/// Launch configuration wizard.
fn configuration_wizard(prompt: &Prompt, path: Option<&str>) -> BoxResult<Config> {
    // Must not fail to continue
    let minimal_config = match prompt.required_config() {
        Ok(c) => match c {
            Some(c) => c,
            None => std::process::exit(1),
        },
        Err(_) => std::process::exit(1),
    };

    // Can fail to continue
    let config = match prompt.optional_config(&minimal_config) {
        Ok(c) => c.unwrap_or(minimal_config),
        Err(_) => minimal_config,
    };

    match config.save(path) {
        Ok(_) => println!("Configuration saved!"),
        Err(e) => {
            error!("Failed to save configuration file: {}", e);
            std::process::exit(1);
        },
    };

    Ok(config)
}

/// Update Slack status based on current location..
fn status_update(client: &SlackStatus, non_interactive: bool) {
    debug!("Requesting public ip...");
    let ip = match client.get_public_ip() {
        Ok(ip) => ip,
        Err(e) => {
            error!("Cannot get public IP: {}", e);
            std::process::exit(1);
        },
    };
    println!("{}: {}",
        style("Current location's public IP").bold(),
        style(ip).cyan()
    );

    debug!("Computing status...");
    let status = client.status_from(&ip);
    let replacer = gh_emoji::Replacer::new();
    println!("{}: {}",
        style("Location's status").bold(),
        style(replacer.replace_all(&format!("{}", status))).yellow()
    );

    // Ask for confirmation before updating status if not in non-interactive mode.
    if non_interactive || Confirmation::new()
        .with_text("Do you want to update your status?")
        .interact()
        .unwrap()
    {
        debug!("Updating Slack status...");
        let res: reqwest::blocking::Response = match client.set_slack_status(&status) {
            Ok(res) => res,
            Err(e) => panic!("Failed to change status: {:?}", e),
        };
        debug!("{:#?}", res);

        let replacer = gh_emoji::Replacer::new();
        println!("{}",
            style(
                replacer.replace_all(":heavy_check_mark: Slack status updated")
            )
            .bold()
            .green()
        );
    } else {
        println!("Nevermind then :(");
        return;
    }
}

/// Print the list of configured locations.
fn list_locations(client: &SlackStatus) {
    debug!("Listing locations...");
    let replacer = gh_emoji::Replacer::new();
    for (n, l) in client.config.locations.iter().enumerate() {
        println!(" {}. {}: {} {}",
            style(n + 1).blue(),
            style(l.ip).cyan(),
            replacer.replace_all(&format!("{}", l.emoji)),
            style(&l.text).yellow(),
        );
    }
}

/// Add (or replace) status for current location.
fn add_location(prompt: &Prompt, client: &SlackStatus, old_config: &Config, custom_path: Option<&str>) {
    debug!("Adding current location...");
    debug!("Requesting public ip...");
    let ip = match client.get_public_ip() {
        Ok(ip) => ip,
        Err(e) => {
            error!("Cannot get public IP: {}", e);
            std::process::exit(1);
        },
    };

    let location = match prompt.add_location(ip) {
        Ok(l) => match l {
            Some(l) => l,
            None => std::process::exit(1),
        },
        Err(_) => std::process::exit(1),
    };

    let replacer = gh_emoji::Replacer::new();
    println!("{}: {} => {} {}",
        style("New location status").bold(),
        style(location.ip).cyan(),
        replacer.replace_all(&format!("{}", location.emoji)),
        style(&location.text).yellow(),
    );

    let mut config = old_config.clone();
    // Remove current status for this location, if any.
    config.locations = old_config.locations.iter()
        .filter(|l| l.ip != ip)
        .map(|l| l.clone()).collect();

    // Add new status for this location.
    config.locations.push(location);

    match config.save(custom_path) {
        Ok(_) => print_configuration_saved(),
        Err(e) => {
            error!("Failed to save configuration file: {}", e);
            std::process::exit(1);
        },
    };
}

/// Remove some configured locations.
fn rm_location(prompt: &Prompt, old_config: &Config, custom_path: Option<&str>) {
    debug!("Prompt for removing a saved location...");

    let checkboxes = &old_config.locations;
    let selections = Checkboxes::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick those you want to remove")
        .items(&checkboxes[..])
        .interact()
        .unwrap();

    if selections.is_empty() {
        println!("{}", style("No modification have been performed.").yellow());
        std::process::exit(0);
    } else {
        let mut tbr = Vec::<&Location>::new();
        let replacer = gh_emoji::Replacer::new();

        println!("{}", style("You selected these locations to be removed:").bold());
        for s in selections {
            println!("  {} => {} {}",
                style(checkboxes[s].ip).cyan(),
                replacer.replace_all(&format!("{}", checkboxes[s].emoji)),
                style(&checkboxes[s].text).yellow(),
            );
            tbr.push(&checkboxes[s]);
        }

        if Confirmation::with_theme(&prompt.theme)
            .with_text("Are you sure you want to remove them?")
            .interact()
            .unwrap()
        {
            let mut config = old_config.clone();

            config.locations = old_config.locations.iter()
                .filter(|l| !tbr.iter()
                    .any(|s| s == l))
                .map(|l| l.clone()).collect();

            match config.save(custom_path) {
                Ok(_) => print_configuration_saved(),
                Err(e) => {
                    error!("Failed to save configuration file: {}", e);
                    std::process::exit(1);
                },
            };
        } else {
            println!("{}", style("No modification have been performed.").yellow());
            std::process::exit(0);
        }
    }
}

/// Manually set current Slack Status.
fn set_status(prompt: &Prompt, client: &SlackStatus) {
    debug!("Manually set status...");

    let status = match prompt.status(":house_with_garden:", "working remotely") {
        Ok(s) => match s {
            Some(s) => s,
            None => std::process::exit(1),
        },
        Err(_) => std::process::exit(1),
    };

    // Ask for confirmation before updating status if not in non-interactive mode.
    if Confirmation::new()
        .with_text("Update your status?")
        .interact()
        .unwrap()
    {
        debug!("Updating Slack status...");
        let res: reqwest::blocking::Response = match client.set_slack_status(&status) {
            Ok(res) => res,
            Err(e) => panic!("Failed to change status: {:?}", e),
        };
        debug!("{:#?}", res);

        let replacer = gh_emoji::Replacer::new();
        println!("{}: {}",
            style("New Slack status").bold(),
            style(replacer.replace_all(&format!("{}", status))).yellow()
        );
    } else {
        println!("Nevermind then :(");
        return;
    }
}

/// Request current Slack status.
fn get_status(client: &SlackStatus) {
    debug!("Requesting your current status...");

    let status = match client.get_slack_status() {
        Ok(res) => match res.text() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to get your Slack status: {:?}", e);
                std::process::exit(1);
            },
        },
        Err(e) => {
            error!("Failed to get your Slack status: {:?}", e);
            std::process::exit(1);
        },
    };

    // TODO: Finish this code as soon as I have the right.
    println!("Your Slack status is: {:#?}", status);
    debug!("{:#?}", status);
}

impl Prompt {
    /// Prompt for required configuration elements.
    fn required_config(&self) -> BoxResult<Option<Config>> {
        println!("Configuration not found!\n");

        if !Confirmation::with_theme(&self.theme)
            .with_text("Do you want to launch the setup wizard?")
            .interact()?
        {
            return Ok(None);
        }

        let token = Input::with_theme(&self.theme)
            .with_prompt("Slack App token")
            .interact()?;

        let ip_request_address = Input::with_theme(&self.theme)
            .with_prompt("Where do you want to request your public IP address?")
            .default("http://ip.clara.net".parse().unwrap())
            .interact()?;

        Ok(Some(Config {
            token: token,
            ip_request_address: Some(ip_request_address),
            defaults: None,
            locations: Vec::<Location>::new(),
        }))
    }

    /// Prompt for optional configuration elements.
    fn optional_config(&self, config: &Config) -> BoxResult<Option<Config>> {
        println!("Default status is used when your current location is unkown.");

        if !Confirmation::with_theme(&self.theme)
            .with_text("Do you want to customize the default status?")
            .interact()?
        {
            return Ok(None);
        }

        let status = match self.status(":mountain_railway:", "commuting") {
            Ok(s) => match s {
                Some(s) => s,
                None => std::process::exit(1),
            },
            Err(_) => std::process::exit(1),
        };

        Ok(Some(Config {
            token: config.token.clone(),
            ip_request_address: config.ip_request_address.clone(),
            defaults: Some(Status {
                text: status.text,
                emoji: status.emoji,
                expire_after_hours: status.expire_after_hours,
            }),
            locations: config.locations.clone(),
        }))
    }

    /// Prompt for setup location.
    fn add_location(&self, ip: IpAddr) -> BoxResult<Option<Location>> {
        println!("{}: {}",
            style("Current location's public IP").bold(),
            style(ip).cyan()
        );

        if !Confirmation::with_theme(&self.theme)
            .with_text("Do you want add/overwrite status for this location?")
            .interact()?
        {
            return Ok(None);
        }

        let status = match self.status(":house_with_garden:", "working remotely") {
            Ok(s) => match s {
                Some(s) => s,
                None => std::process::exit(1),
            },
            Err(_) => std::process::exit(1),
        };

        Ok(Some(Location {
            ip: ip,
            text: status.text,
            emoji: status.emoji,
            expire_after_hours: status.expire_after_hours,
        }))
    }

    /// Prompt for status.
    fn status(&self, default_emoji: &str, default_text: &str) -> BoxResult<Option<Status>> {
        let emoji = Input::with_theme(&self.theme)
            .with_prompt("emoji")
            .default(default_emoji.parse().unwrap())
            .interact()?;

        let text = Input::with_theme(&self.theme)
            .with_prompt("status")
            .default(default_text.parse().unwrap())
            .interact()?;

        let expires = Select::with_theme(&self.theme)
            .with_prompt("Status expires after")
            .default(0)
            .item("1 hour")
            .item("1 day")
            .item("never")
            .interact()?;

        Ok(Some(Status {
            text: text,
            emoji: emoji,
            expire_after_hours: match expires {
                0 => Some(1),
                1 => Some(24),
                _ => None,
            },
        }))
    }
}

/// Setup logger.
fn setup_logger(log_level: log::LevelFilter) -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log_level)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

fn print_configuration_saved() {
    let replacer = gh_emoji::Replacer::new();
    println!("{}",
        style(
            replacer.replace_all(":heavy_check_mark: Configuration saved!")
        )
        .bold()
        .green()
    )
}
