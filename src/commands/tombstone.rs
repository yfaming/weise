use crate::commands::DataDirConfig;
use crate::storage::Storage;
use log::info;

#[derive(Debug, clap::Parser)]
pub struct Config {
    #[clap(flatten)]
    data_dir_config: DataDirConfig,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Parser)]
enum Command {
    Add(AddConfig),
    Clear,
}

#[derive(Debug, clap::Parser)]
pub struct AddConfig {
    items: Vec<String>,
}

pub async fn command(config: Config) -> Result<(), anyhow::Error> {
    let storage = config.data_dir_config.storage()?;
    match config.command {
        Command::Add(add_config) => tombstone_add(storage, add_config)?,
        Command::Clear => storage.post_tombstones().delete_all()?,
    }
    Ok(())
}

fn tombstone_add(storage: Storage, config: AddConfig) -> Result<(), anyhow::Error> {
    for item in &config.items {
        if item.starts_with("https://weibo.com") {
            let post = match storage.posts().get_by_url(item)? {
                Some(post) => post,
                None => {
                    info!("post not found with url = {}", item);
                    break;
                }
            };
            storage.post_tombstones().add(&post)?;
            info!("added post with url = {}", item);
        } else if let Ok(post_id) = item.parse::<i64>() {
            let post = match storage.posts().get_by_id(post_id)? {
                Some(post) => post,
                None => {
                    info!("post not found with id = {}", item);
                    break;
                }
            };
            storage.post_tombstones().add(&post)?;
            info!("added post with id = {}", item);
        } else {
            info!("invalid post id or url: {}", item);
            break;
        }
    }
    Ok(())
}
