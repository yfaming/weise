use crate::commands::DataDirConfig;
use crate::storage::Storage;
use log::error;

#[derive(Debug, clap::Parser)]
pub struct Config {
    #[clap(flatten)]
    data_dir_config: DataDirConfig,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Parser)]
enum Command {
    Set(SetConfig),
    Show,
}

#[derive(Debug, clap::Parser)]
pub struct SetConfig {
    items: Vec<String>,
}

pub async fn command(config: Config) -> Result<(), anyhow::Error> {
    config.data_dir_config.ensure_data_dir_exists()?;
    let storage = config.data_dir_config.storage()?;
    match config.command {
        Command::Set(set_config) => settings_set(storage, set_config)?,
        Command::Show => settings_show(storage)?,
    }
    Ok(())
}

fn settings_set(storage: Storage, config: SetConfig) -> Result<(), anyhow::Error> {
    for item in &config.items {
        let kv: Vec<&str> = item.split('=').collect();
        if kv.len() != 2 {
            error!("settings should be as follows: <name>=<value>");
            continue;
        }

        let name = kv[0].trim();
        match name {
            "max_page" => {
                let value = kv[1].trim();
                let value: u32 = value.parse().map_err(|_e| {
                    anyhow::format_err!("max_page should be an integer, instead of {}", value)
                })?;
                storage.settings().set_max_page(value)?;
            }
            _ => {
                return Err(anyhow::format_err!("settings not supported: {}", name));
            }
        }
    }
    Ok(())
}

fn settings_show(storage: Storage) -> Result<(), anyhow::Error> {
    match storage.settings().get_max_page()? {
        Some(max_page) => println!("max_page = {}", max_page),
        None => println!("max_page = <unset>"),
    }
    Ok(())
}
