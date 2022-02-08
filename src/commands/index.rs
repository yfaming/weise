use crate::commands::DataDirConfig;
use crate::weibo::post::Post;
use log::info;
use std::fs;

#[derive(Debug, clap::Parser)]
pub struct Config {
    #[clap(flatten)]
    data_dir_config: DataDirConfig,
}

pub async fn command(config: Config) -> Result<(), anyhow::Error> {
    let index_dir = config.data_dir_config.index_dir();
    if index_dir.exists() {
        info!("clear index_dir: {}", index_dir.display());
        fs::remove_dir_all(index_dir)?;
    }
    config.data_dir_config.ensure_data_dir_exists()?;

    let storage = config.data_dir_config.storage()?;
    let indexer = config.data_dir_config.weibo_indexer()?;
    let tombstones = storage.post_tombstones().all_post_ids()?;

    let limit = 10000;
    let mut post_id = 0;
    loop {
        let posts = storage.posts().get_posts(post_id, limit)?;

        let should_continue = posts.len() == limit;
        if should_continue {
            post_id = posts[posts.len() - 1].id;
        }

        let filtered_posts: Vec<Post> = posts
            .into_iter()
            .filter(|p| !tombstones.contains(&p.id))
            .collect();

        if !filtered_posts.is_empty() {
            indexer.index_weibo_posts(&filtered_posts)?;
            info!("indexed {} weibo posts", filtered_posts.len());
        }

        if !should_continue {
            break;
        }
    }
    Ok(())
}
