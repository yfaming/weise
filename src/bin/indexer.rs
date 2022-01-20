use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use weise::index::WeiboIndexer;
use weise::weibo::raw::RawPost;

const INDEX_DIR: &str = ".index";

fn main() -> Result<(), anyhow::Error> {
    index_all_posts()?;
    Ok(())
}

pub fn index_all_posts() -> Result<(), anyhow::Error> {
    #[derive(Deserialize)]
    pub struct FavRes {
        // ok: i32,
        data: Vec<RawPost>,
    }

    let weibo_indexer = WeiboIndexer::with_index_dir(INDEX_DIR)?;
    let mut posts = vec![];
    let mut post_ids = HashSet::new();
    const MAX_PAGE_ID: u32 = 1792;
    for page_id in 1..=MAX_PAGE_ID {
        println!("=============== page_id: {}", page_id);

        let file_name = format!("data/page_{:04}.json", page_id);
        let content = fs::read_to_string(file_name)?;
        let res: FavRes = serde_json::from_str(&content)?;

        for raw in res.data {
            let post = raw.normalize();
            if post.is_valid() && !post_ids.contains(&post.id) {
                post_ids.insert(post.id);
                posts.push(post);
            }
        }

        if posts.len() >= 5000 {
            weibo_indexer.index_weibo_posts(&posts)?;
            posts = vec![]
        }
    }
    weibo_indexer.index_weibo_posts(&posts)?;

    Ok(())
}
