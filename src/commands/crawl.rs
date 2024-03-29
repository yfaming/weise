use crate::chromedriver::{start_chromedriver, ChromeDriverProcess};
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

    let weibo_client = WeiboClient::login().await?;
    for page_id in config.start_page..=end_page {
        let posts = get_favs_by_page_with_retry(&weibo_client, page_id, 3).await?;
        let mut valid_posts = vec![];
        for p in posts {
            if p.is_valid() {
                valid_posts.push(p);
            } else {
                info!("invalid post, id: {}, url: {}", p.id, p.url());
            }
        }
        info!("page={}, valid post count: {}", page_id, valid_posts.len());
        storage.posts().batch_add(&valid_posts)?;
    }

    weibo_client.close().await?;
    Ok(())
}

async fn get_favs_by_page_with_retry(
    weibo_client: &WeiboClient,
    page_id: u32,
    retries: u32,
) -> Result<Vec<Post>, anyhow::Error> {
    for i in 0..retries {
        match weibo_client.get_favs_by_page(page_id).await {
            Ok(posts) => return Ok(posts),
            Err(e) if i == retries - 1 => return Err(e.into()),
            _ => continue,
        }
    }
    unreachable!()
}

pub struct WeiboClient {
    #[allow(unused)]
    chromedriver: ChromeDriverProcess,
    driver: WebDriver,
}

impl WeiboClient {
    pub async fn login() -> Result<WeiboClient, anyhow::Error> {
        let chromedriver = start_chromedriver(4444)?;
        let cap = DesiredCapabilities::chrome();
        let driver = WebDriver::new(&chromedriver.server_url(), cap).await?;
        driver.goto("https://weibo.com/").await?;
        // TODO: 改为等待登录成功
        sleep(Duration::from_secs(20));

        Ok(WeiboClient {
            chromedriver,
            driver,
        })
    }

    pub async fn close(self) -> Result<(), anyhow::Error> {
        self.driver.quit().await?;
        Ok(())
    }

    pub async fn get_favs_by_page(&self, page_id: u32) -> Result<Vec<Post>, anyhow::Error> {
        self.driver
            .goto(format!(
                "https://weibo.com/ajax/favorites/all_fav?page={}",
                page_id
            ))
            .await?;

        let content = self.driver.find(By::Css("pre")).await?.text().await?;
        let mut posts = vec![];
        let res: FavResponse = serde_json::from_str(&content)?;
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
