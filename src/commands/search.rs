use crate::commands::DataDirConfig;
use crate::index::{SearchedWeiboPost, WeiboIndexer, WeiboSearchParams};

#[derive(Debug, clap::Parser)]
pub struct Config {
    #[clap(flatten)]
    data_dir_config: DataDirConfig,

    query: Option<String>,
    #[clap(long)]
    media_type: Option<u8>,
    #[clap(short, long)]
    user: Option<String>,
    #[clap(short, long, default_value = "10")]
    limit: usize,
}

pub async fn command(config: Config) -> Result<(), anyhow::Error> {
    config.data_dir_config.ensure_data_dir_exists()?;

    let params = WeiboSearchParams {
        media_type: config.media_type,
        user: config.user,
        query: config.query,
    };

    let weibo_indexer = WeiboIndexer::with_index_dir(config.data_dir_config.index_dir())?;
    let posts = weibo_indexer.search(&params, config.limit)?;
    for post in posts {
        prettify_post(&post);
    }
    Ok(())
}

fn prettify_post(post: &SearchedWeiboPost) {
    let text = post.text.replace("\n", " ");
    let mut s = format!("{}\n@{}: {}", post.url, post.user, text);
    if let Some(retweeted_user) = &post.retweeted_user {
        let tmp = format!("  @{}: ", retweeted_user);
        s.push_str(&tmp);
    }
    if let Some(retweeted_text) = &post.retweeted_text {
        let tmp = retweeted_text.replace("\n", " ");
        s.push_str(&tmp);
    }
    println!("{}\n", s);
}
