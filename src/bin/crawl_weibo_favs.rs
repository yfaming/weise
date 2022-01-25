use serde::Deserialize;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;
use structopt::StructOpt;
use thirtyfour::prelude::*;
use weise::weibo::post::Post;
use weise::weibo::raw::RawPost;

const DIR: &str = "data";

#[derive(Debug, StructOpt)]
#[structopt(name = "crawl_weibo_favs", about = "crawl weibo favs")]
struct Opt {
    #[structopt(long)]
    start_page: u32,
    #[structopt(long)]
    end_page: u32,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let opt = Opt::from_args();

    let weibo_client = WeiboClient::login().await?;

    for page_id in opt.start_page..=opt.end_page {
        let page_content = weibo_client.get_favs_raw(page_id).await?;
        let file_name = format!("{}/page_{:04}.json", DIR, page_id);
        let mut f = std::fs::File::create(file_name)?;
        f.write_all(page_content.as_bytes())?;
    }

    weibo_client.close().await?;
    Ok(())
}

pub struct WeiboClient {
    driver: WebDriver,
}

impl WeiboClient {
    pub async fn login() -> Result<WeiboClient, anyhow::Error> {
        // 在此之前，需要先通过 chromedriver --port=4444 运行 chromedriver
        let caps = DesiredCapabilities::chrome();
        let driver = WebDriver::new("http://localhost:4444", &caps).await?;
        driver.get("https://weibo.com/").await?;
        // TODO: 改为等待登录成功
        sleep(Duration::from_secs(20));

        Ok(WeiboClient { driver })
    }

    pub async fn close(&self) -> Result<(), anyhow::Error> {
        self.driver.close().await?;
        Ok(())
    }

    pub async fn get_favs_raw(&self, page_id: u32) -> Result<String, anyhow::Error> {
        self.driver
            .get(format!(
                "https://weibo.com/ajax/favorites/all_fav?page={}",
                page_id
            ))
            .await?;
        let content = self.driver.page_source().await?;
        let start = content
            .find('{')
            .ok_or(anyhow::format_err!("invalid result"))?;
        let end = content
            .find("</pre></body></html>")
            .ok_or(anyhow::format_err!("invalid result"))?;

        let val: serde_json::Value = serde_json::from_str(&content[start..end])?;
        let pretty_str = serde_json::to_string_pretty(&val)?;
        Ok(pretty_str)
    }

    pub async fn get_favs_by_page(&self, page_id: u32) -> Result<Vec<Post>, anyhow::Error> {
        self.driver
            .get(format!(
                "https://weibo.com/ajax/favorites/all_fav?page={}",
                page_id
            ))
            .await?;
        let content = self.driver.page_source().await?;
        let start = content
            .find('{')
            .ok_or(anyhow::format_err!("invalid result"))?;
        let end = content
            .find("</pre></body></html>")
            .ok_or(anyhow::format_err!("invalid result"))?;

        let mut posts = vec![];
        let res: FavResponse = serde_json::from_str(&content[start..end])?;
        for rp in res.data {
            let p = rp.normalize();
            posts.push(p);
        }
        Ok(posts)
    }
}

#[derive(Deserialize)]
struct FavResponse {
    // ok: i32,
    data: Vec<RawPost>,
}
