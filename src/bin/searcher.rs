use structopt::StructOpt;
use weise::index::{WeiboIndexer, WeiboSearchParams, SearchedWeiboPost};

const INDEX_DIR: &str = ".index";

fn main() -> Result<(), anyhow::Error> {
    let opt = Opt::from_args();

    let params = WeiboSearchParams {
        media_type: opt.media_type,
        query: opt.query,
    };
    let limit = opt.limit.unwrap_or(10);

    let weibo_indexer = WeiboIndexer::with_index_dir(INDEX_DIR)?;
    let posts = weibo_indexer.search(&params, limit)?;
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
    println!("{}", s);
}

#[derive(Debug, StructOpt)]
#[structopt(name = "xunfei", about = "讯飞语音转写命令行工具")]
struct Opt {
    #[structopt(long)]
    query: Option<String>,
    #[structopt(long)]
    media_type: Option<u8>,
    #[structopt(long)]
    limit: Option<usize>,
}
