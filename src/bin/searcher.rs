use structopt::StructOpt;
use weise::index::{SearchedWeiboPost, WeiboIndexer, WeiboSearchParams};

fn main() -> Result<(), anyhow::Error> {
    let opt = Opt::from_args();

    let params = WeiboSearchParams {
        media_type: opt.media_type,
        query: opt.query,
    };
    let limit = opt.limit.unwrap_or(10);

    let index_dir = dirs::home_dir().unwrap().join(".weibofav_index");
    let weibo_indexer = WeiboIndexer::with_index_dir(&index_dir)?;
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
    println!("{}\n", s);
}

#[derive(Debug, StructOpt)]
#[structopt(name = "weibofav", about = "微博搜索")]
struct Opt {
    query: Option<String>,
    #[structopt(long)]
    media_type: Option<u8>,
    #[structopt(long)]
    limit: Option<usize>,
}
