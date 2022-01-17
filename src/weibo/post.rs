use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub struct Post {
    pub id: i64,
    pub mblogid: String,
    pub user: User,
    pub text_raw: String,
    pub is_long_text: bool,

    pub media_asset: MediaAsset,

    // 这个字段有点麻烦。微博 API 返回的内容形如: Sun Jan 09 11:50:55 +0800 2022
    pub created_at: DateTime<FixedOffset>,

    pub retweeted_post: Option<Box<Post>>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct User {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub screen_name: String,
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum MediaType {
    Text = 0,
    Picture = 1,
    Video = 2,
}

#[derive(Debug, PartialEq, Clone)]
pub enum MediaAsset {
    None,
    Pictures(Vec<String>),
    Video(VideoEntry),
}

#[derive(Debug, PartialEq, Clone)]
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
