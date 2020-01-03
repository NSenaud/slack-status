#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

use clap::App;

use slack_status::*;

fn main() {
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

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

    debug!("Reading configuration...");
    let config = match Config::read(matches.value_of("config")) {
        Some(config) => config,
        None => {
            println!("Configuration file not found!");
            let config_dir = get_config_dir().unwrap();
            create_default_config(&config_dir).unwrap();
            println!("Sample configuration file created at: {:?}", config_dir);
            println!("Please edit and add your legacy Slack token.");
            std::process::exit(1);
        }
    };

    let client = match SlackStatus::from(config) {
        Ok(c) => c,
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    };

    info!("Requesting public ip...");
    let ip = match client.get_public_ip() {
        Ok(ip) => ip,
        Err(e) => {
            error!("Cannot parse IP: {}", e);
            return;
        },
    };
    info!("Public IP is: {}", ip);

    info!("Computing status...");
    let status = client.status_from(&ip);
    info!("Status is: {} {}", status.emoji, status.text);

    info!("Updating Slack status...");
    let res: reqwest::Response = match client.set_slack_status(status) {
        Ok(res) => res,
        Err(e) => panic!("Failed to change status: {:?}", e),
    };

    debug!("{:#?}", res);
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
