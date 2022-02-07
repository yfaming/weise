use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Post {
    pub id: i64,
    pub mblogid: String,
    pub user: User,
    pub text_raw: String,
    pub is_long_text: bool,
    pub media_asset: MediaAsset,
    pub created_at: DateTime<FixedOffset>,

    pub retweeted_post: Option<Box<Post>>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Default)]
pub struct User {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub screen_name: String,
}

#[derive(Debug, PartialEq, Clone, Copy, Deserialize, Serialize)]
#[repr(u8)]
pub enum MediaType {
    Text = 0,
    Picture = 1,
    Video = 2,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub enum MediaAsset {
    None,
    Pictures(Vec<String>),
    Video(VideoEntry),
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct VideoEntry {
    pub url: String,
    pub duration_secs: u32,
    pub cover_picture_url: String,
}

impl Post {
    pub fn url(&self) -> String {
        format!("https://weibo.com/{}/{}", self.user.id, self.mblogid)
    }

    pub fn media_type(&self) -> MediaType {
        match &self.retweeted_post {
            Some(p) => p.media_type(),
            None => self.media_asset.media_type(),
        }
    }

    pub fn is_retweet(&self) -> bool {
        self.retweeted_post.is_some()
    }

    pub fn is_valid(&self) -> bool {
        if self.user.id == 0 {
            return false;
        }
        if let Some(retweeted_post) = &self.retweeted_post {
            if !retweeted_post.is_valid() {
                return false;
            }
        }

        true
    }
}

impl User {
    pub fn profile_url(&self) -> String {
        format!("https://weibo.com/u/{}", self.id)
    }
}

impl MediaAsset {
    fn media_type(&self) -> MediaType {
        match self {
            MediaAsset::None => MediaType::Text,
            MediaAsset::Pictures(_) => MediaType::Picture,
            MediaAsset::Video(_) => MediaType::Video,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::offset::FixedOffset;
    use chrono::TimeZone;

    #[test]
    fn test_post_serde_roundtrip() -> Result<(), anyhow::Error> {
        let user = User {
            id: 1773116334,
            screen_name: "zhh-4096".to_string(),
        };
        let post = Post {
            id: 4723695598438753,
            mblogid: "L9WqHzpiV".to_string(),
            user,
            text_raw: "今年我一定会开一家新公司以 GraalVM 为工具研发几个产品，目前产品思路逐渐清晰，长中短期都有，不会再像过去十年研究数据库那么耗时了，搞数据库基础理论创新实在是太硬核了，没有好的思路半年都没啥进展。[允悲] ​​​".to_string(),
            is_long_text: false,
            media_asset: MediaAsset::None,
            created_at: FixedOffset::east(8 * 3600)
                .ymd(2022, 1, 9)
                .and_hms(11, 50, 55),
            retweeted_post: None,
        };
        let s = serde_json::to_string_pretty(&post)?;

        let post2: Post = serde_json::from_str(&s)?;
        assert_eq!(post, post2);
        Ok(())
    }

    #[test]
    fn test_deserialize_user() -> Result<(), anyhow::Error> {
        let json = r#"{
            "id": 2131170823,
            "idstr": "2131170823",
            "pc_new": 7,
            "screen_name": "梁博penny",
            "profile_image_url": "https://tva2.sinaimg.cn/crop.0.0.180.180.50/7f071607jw1e8qgp5bmzyj2050050aa8.jpg?KID=imgbed,tva&Expires=1641705697&ssig=Wcu7q1byxZ",
            "profile_url": "/u/2131170823",
            "verified": false,
            "verified_type": -1,
            "domain": "thuirdb",
            "weihao": "",
            "avatar_large": "https://tva2.sinaimg.cn/crop.0.0.180.180.180/7f071607jw1e8qgp5bmzyj2050050aa8.jpg?KID=imgbed,tva&Expires=1641705697&ssig=7nt1ZUb2Y%2F",
            "avatar_hd": "https://tva2.sinaimg.cn/crop.0.0.180.180.1024/7f071607jw1e8qgp5bmzyj2050050aa8.jpg?KID=imgbed,tva&Expires=1641705697&ssig=ve7RukJYex",
            "follow_me": false,
            "following": true,
            "mbrank": 3,
            "mbtype": 12,
            "planet_video": false
        }"#;

        let user: User = serde_json::from_str(json)?;
        let expected_user = User {
            id: 2131170823,
            screen_name: "梁博penny".to_string(),
        };
        assert_eq!(user, expected_user);
        assert_eq!(user.profile_url(), "https://weibo.com/u/2131170823");
        Ok(())
    }
}
