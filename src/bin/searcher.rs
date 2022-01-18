use structopt::StructOpt;
use weise::index::{WeiboIndexer, WeiboSearchParams};

const INDEX_DIR: &str = ".index";

fn main() -> Result<(), anyhow::Error> {
    let opt = Opt::from_args();

    let params = WeiboSearchParams {
        media_type: opt.media_type,
        query: opt.query,
    };
    let limit = opt.limit.unwrap_or(10);

    let weibo_indexer = WeiboIndexer::with_index_dir(INDEX_DIR)?;
    let res = weibo_indexer.search(&params, limit)?;
    for entry in res {
        println!("{}", entry);
    }

    Ok(())
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
