use chrono::Local;
use colored::Colorize;
use log::{error, info, Record};
use pretty_env_logger::env_logger::fmt::Color;
use std::{default, io::Write};
use uniswap_v3_mev::{mempool::listener::mempool_listener, types::settings::Settings};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    pretty_env_logger::env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("ethers_providers", log::LevelFilter::Warn)
        .format(|f, record| {
            let level = record.level();
            let color = match level {
                log::Level::Error => Color::Red,
                log::Level::Warn => Color::Yellow,
                log::Level::Info => Color::Green,
                log::Level::Debug => Color::Blue,
                log::Level::Trace => Color::Magenta,
            };

            let mut style = f.style();
            style.set_color(color).set_bold(true);

            let timestamp = Local::now().format("%I:%M:%S%.3f %p");

            writeln!(
                f,
                "{} {} {} {}",
                style.value(level),
                timestamp,
                "⮞ ".bold().bright_black(),
                record.args()
            )
        })
        .init();
    // Use new() for reading the settings from config.toml file and read_config for creating and reading the settings from the config.json file
    let settings = Settings::new()?;
    info!("{:#?}", settings);

    let _ = match mempool_listener(settings).await {
        Ok(_) => (),
        Err(e) => error!("Error starting mempool listener: {}", e),
    };

    Ok(())
}
