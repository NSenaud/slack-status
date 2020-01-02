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

    println!("{}", log_level);
    match setup_logger(log_level) {
        Ok(_) => debug!("Logger set up"),
        Err(e) => println!("Failed to setup logger: {}", e),
    };

    debug!("Reading configuration...");
    let config = match get_config() {
        Some(config) => config,
        None => {
            println!("Configuration file not found!");
            let config_dir = get_config_dir().unwrap();
            create_default_config(&config_dir).unwrap();
            println!("Sample configuration file created in {:?}, please edit and add your legacy Slack token.", config_dir);
            std::process::exit(1);
        }
    };

    debug!("Checking Slack legacy token is not empty...");
    let token = if config.token != "" {
        config.token.clone()
    } else {
        println!("You must copy your Slack legacy token to configuration file.");
        std::process::exit(1);
    };

    info!("Requesting public ip...");
    let ip = match get_public_ip() {
        Ok(ip) => ip,
        Err(e) => {
            error!("Cannot parse IP: {}", e);
            return;
        },
    };
    info!("Public IP is: {}", ip);

    info!("Computing status...");
    let status = get_status_from(config, &ip);
    info!("Status is: {} {}", status.emoji, status.text);

    info!("Updating Slack status...");
    let res: reqwest::Response = match set_slack_status(status, token)
    {
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
