use crate::commands::DataDirConfig;
use crate::weibo::post::Post;
use crate::weibo::raw::RawPost;
use log::info;
use serde::Deserialize;
use std::thread::sleep;
use std::time::Duration;
use thirtyfour::prelude::*;

#[derive(Debug, clap::Parser)]
pub struct Config {
    #[clap(flatten)]
    data_dir_config: DataDirConfig,

    #[clap(long, default_value = "1")]
    start_page: u32,
    #[clap(long)]
    end_page: Option<u32>,

    #[clap(
        long,
        help = "形如 http://localhost:4444。可通过 chromedriver --port=4444 运行 chromedriver"
    )]
    webdriver_url: String,
}

pub async fn command(config: Config) -> Result<(), anyhow::Error> {
    config.data_dir_config.ensure_data_dir_exists()?;
    let storage = config.data_dir_config.storage()?;

    let end_page = match config.end_page {
        Some(end_page) => end_page,
        None => storage
            .settings()
            .get_max_page()?
            .expect("max_page not set"),
    };
    info!("crawl from page={} to {}", config.start_page, end_page);

    if config.start_page <= 1 {
        info!(
            "delete all local data first, since we're starting from page={}",
            config.start_page
        );
        storage.posts().delete_all()?;
    }

    let weibo_client = WeiboClient::login(&config.webdriver_url).await?;
    for page_id in config.start_page..=end_page {
        let posts = weibo_client.get_favs_by_page(page_id).await?;
        info!("page={}, post count: {}", page_id, posts.len());
        storage.posts().batch_add(&posts)?;
    }

    weibo_client.close().await?;
    Ok(())
}

pub struct WeiboClient {
    driver: WebDriver,
}

impl WeiboClient {
    pub async fn login(webdriver_url: &str) -> Result<WeiboClient, anyhow::Error> {
        // webdriver_url 形如 http://localhost:4444
        // 在此之前，需要先通过如 chromedriver --port=4444 的命令运行 chromedriver
        let caps = DesiredCapabilities::chrome();
        let driver = WebDriver::new(webdriver_url, &caps).await?;
        driver.get("https://weibo.com/").await?;
        // TODO: 改为等待登录成功
        sleep(Duration::from_secs(20));

        Ok(WeiboClient { driver })
    }

    pub async fn close(&self) -> Result<(), anyhow::Error> {
        self.driver.close().await?;
        Ok(())
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
