#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

use std::net::IpAddr;

use clap::App;
use console::Style;
use dialoguer::{theme::ColorfulTheme, Checkboxes, Confirmation, Input, Select};

use slack_status::*;

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

    // Configuration reading, if configuration is not found launch wizard.
    let config = match Config::read(matches.value_of("config")) {
        Ok(c) => match c {
            Some(c) => c,
            None => match configuration_wizard(matches.value_of("config")) {
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

    // Init SlackStatus client.
    let client = match SlackStatus::from(&config) {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };

    if first_init {
        add_location(&client, &config, matches.value_of("config"));
    }

    // Arguments reading
    if let Some(submatches) = matches.subcommand_matches("location") {
        if let Some(_) = submatches.subcommand_matches("list") {
            list_locations(&client);
        } else if let Some(_) = submatches.subcommand_matches("add") {
            add_location(&client, &config, matches.value_of("config"));
        } else if let Some(_) = submatches.subcommand_matches("rm") {
            rm_location(&config, matches.value_of("config"));
        } else {
            App::from_yaml(yaml).help_message("Wrong parameter");
        }
    } else if let Some(submatches) = matches.subcommand_matches("status") {
        if let Some(_) = submatches.subcommand_matches("get") {
            get_status(&client);
        } else if let Some(_) = submatches.subcommand_matches("set") {
            set_status(&client);
        }
    } else {
        status_update(&client, matches.is_present("noninteractive"));
    }

    std::process::exit(0)
}

fn configuration_wizard(path: Option<&str>) -> BoxResult<Config> {
    let minimal_config = match minimal_config_prompt() {
        Ok(c) => match c {
            Some(c) => c,
            None => std::process::exit(1),
        },
        Err(_) => std::process::exit(1),
    };

    let config = match config_defaults_prompt(&minimal_config) {
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

fn minimal_config_prompt() -> BoxResult<Option<Config>> {
    let theme = ColorfulTheme {
        values_style: Style::new().yellow().dim(),
        indicator_style: Style::new().yellow().bold(),
        yes_style: Style::new().yellow().dim(),
        no_style: Style::new().yellow().dim(),
        ..ColorfulTheme::default()
    };
    println!("Configuration not found!\n");

    if !Confirmation::with_theme(&theme)
        .with_text("Do you want to launch the setup wizard?")
        .interact()?
    {
        return Ok(None);
    }

    let token = Input::with_theme(&theme)
        .with_prompt("Slack App token")
        .interact()?;

    let ip_request_address = Input::with_theme(&theme)
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

fn config_defaults_prompt(config: &Config) -> BoxResult<Option<Config>> {
    let theme = ColorfulTheme {
        values_style: Style::new().yellow().dim(),
        indicator_style: Style::new().yellow().bold(),
        yes_style: Style::new().yellow().dim(),
        no_style: Style::new().yellow().dim(),
        ..ColorfulTheme::default()
    };
    println!("Default status is used when your current location is unkown.");

    if !Confirmation::with_theme(&theme)
        .with_text("Do you want to customize the default status?")
        .interact()?
    {
        return Ok(None);
    }

    let emoji = Input::with_theme(&theme)
        .with_prompt("emoji")
        .default(":mountain_railway:".parse().unwrap())
        .interact()?;

    let status = Input::with_theme(&theme)
        .with_prompt("status")
        .default("commuting".parse().unwrap())
        .interact()?;

    let expires = Select::with_theme(&theme)
        .with_prompt("Status expires after")
        .default(0)
        .item("1 hour")
        .item("1 day")
        .item("never")
        .interact()?;

    Ok(Some(Config {
        token: config.token.clone(),
        ip_request_address: config.ip_request_address.clone(),
        defaults: Some(Status {
            text: status,
            emoji: emoji,
            expire_after_hours: match expires {
                0 => Some(1),
                1 => Some(24),
                _ => None,
            },
        }),
        locations: config.locations.clone(),
    }))
}

fn list_locations(client: &SlackStatus) {
    debug!("Listing locations...");
    for location in client.config.locations.iter() {
        println!(" - {}", location);
    }
}

fn add_location(client: &SlackStatus, old_config: &Config, custom_path: Option<&str>) {
    debug!("Adding current location...");
    debug!("Requesting public ip...");
    let ip = match client.get_public_ip() {
        Ok(ip) => ip,
        Err(e) => {
            error!("Cannot get public IP: {}", e);
            std::process::exit(1);
        },
    };
    info!("Public IP is: {}", ip);

    let location = match add_location_prompt(ip) {
        Ok(loc) => loc,
        Err(_) => std::process::exit(1),
    };

    let mut config = old_config.clone();
    // Remove current status for this location, if any.
    config.locations = old_config.locations.iter()
        .filter(|l| l.ip != ip)
        .map(|l| l.clone()).collect();

    // Add new status for this location.
    config.locations.push(location.unwrap());

    match config.save(custom_path) {
        Ok(_) => println!("Configuration saved!"),
        Err(e) => {
            error!("Failed to save configuration file: {}", e);
            std::process::exit(1);
        },
    };
}

fn add_location_prompt(ip: IpAddr) -> BoxResult<Option<Location>> {
    let theme = ColorfulTheme {
        values_style: Style::new().yellow().dim(),
        indicator_style: Style::new().yellow().bold(),
        yes_style: Style::new().yellow().dim(),
        no_style: Style::new().yellow().dim(),
        ..ColorfulTheme::default()
    };
    println!("Your current location's public IP is: {}\n", ip);

    if !Confirmation::with_theme(&theme)
        .with_text("Do you want add a status or overwrite an existing one for this location?")
        .interact()?
    {
        return Ok(None);
    }

    let emoji = Input::with_theme(&theme)
        .with_prompt("emoji")
        .default(":house_with_garden:".parse().unwrap())
        .interact()?;

    let status = Input::with_theme(&theme)
        .with_prompt("status")
        .default("working remotely".parse().unwrap())
        .interact()?;

    let expires = Select::with_theme(&theme)
        .with_prompt("Status expires after")
        .default(0)
        .item("1 hour")
        .item("1 day")
        .item("never")
        .interact()?;

    Ok(Some(Location {
        ip: ip,
        text: status,
        emoji: emoji,
        expire_after_hours: match expires {
            0 => Some(1),
            1 => Some(24),
            _ => None,
        },
    }))
}

fn rm_location(old_config: &Config, custom_path: Option<&str>) {
    debug!("Prompt for removing a saved location...");

    let checkboxes = &old_config.locations;
    let selections = Checkboxes::with_theme(&ColorfulTheme::default())
        .with_prompt("Pick those you want to remove")
        .items(&checkboxes[..])
        .interact()
        .unwrap();

    if selections.is_empty() {
        println!("No modification have been performed.");
        std::process::exit(0);
    } else {
        let mut tbr = Vec::<&Location>::new();
        println!("You selected these locations to be removed:");
        for selection in selections {
            println!("  {}", checkboxes[selection]);
            tbr.push(&checkboxes[selection]);
        }

        if Confirmation::new()
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
                Ok(_) => println!("Configuration saved!"),
                Err(e) => {
                    error!("Failed to save configuration file: {}", e);
                    std::process::exit(1);
                },
            };
        } else {
            println!("No modification have been performed.");
            std::process::exit(0);
        }
    }
}

fn status_update(client: &SlackStatus, non_interactive: bool) {
    debug!("Requesting public ip...");
    let ip = match client.get_public_ip() {
        Ok(ip) => ip,
        Err(e) => {
            error!("Cannot get public IP: {}", e);
            std::process::exit(1);
        },
    };
    println!("Your current location's public IP is: {}", ip);

    debug!("Computing status...");
    let status = client.status_from(&ip);
    println!("Location's status is: {}", status);

    // Ask for confirmation before updating status if not in non-interactive mode.
    if non_interactive || Confirmation::new()
        .with_text("Do you want to update your status?")
        .interact()
        .unwrap()
    {
        debug!("Updating Slack status...");
        let res: reqwest::blocking::Response = match client.set_slack_status(status) {
            Ok(res) => res,
            Err(e) => panic!("Failed to change status: {:?}", e),
        };
        println!("Your Slack status have been updated!");
        debug!("{:#?}", res);
    } else {
        println!("Nevermind then :(");
        return;
    }
}

fn set_status(client: &SlackStatus) {
    debug!("Manually set status...");

    let status = match set_status_prompt() {
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
        let res: reqwest::blocking::Response = match client.set_slack_status(status) {
            Ok(res) => res,
            Err(e) => panic!("Failed to change status: {:?}", e),
        };
        println!("Your Slack status have been updated!");
        debug!("{:#?}", res);
    } else {
        println!("Nevermind then :(");
        return;
    }
}

fn set_status_prompt() -> BoxResult<Option<Status>> {
    let theme = ColorfulTheme {
        values_style: Style::new().yellow().dim(),
        indicator_style: Style::new().yellow().bold(),
        yes_style: Style::new().yellow().dim(),
        no_style: Style::new().yellow().dim(),
        ..ColorfulTheme::default()
    };

    let emoji = Input::with_theme(&theme)
        .with_prompt("emoji")
        .default(":house_with_garden:".parse().unwrap())
        .interact()?;

    let status = Input::with_theme(&theme)
        .with_prompt("status")
        .default("working remotely".parse().unwrap())
        .interact()?;

    let expires = Select::with_theme(&theme)
        .with_prompt("Status expires after")
        .default(0)
        .item("1 hour")
        .item("1 day")
        .item("never")
        .interact()?;

    Ok(Some(Status {
        text: status,
        emoji: emoji,
        expire_after_hours: match expires {
            0 => Some(1),
            1 => Some(24),
            _ => None,
        },
    }))
}

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
