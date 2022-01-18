use crate::weibo::post::{MediaAsset, Post, User, VideoEntry};
use chrono::{DateTime, FixedOffset};
use regex::Regex;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::HashMap;

// 微博的收藏接口返回的字段，有些特别之处需要注意。
// 首先，转发微博时，会形成嵌套结构，被转发的微博(原微博)在 retweeted_status 字段之中。
// 但是，嵌套的 retweeted_status 里的结构，与外层结构又非完全一致。
// 某些信息，即使是属于嵌套结构中的信息，也只会保存在外层的字段中。
// 1)图片微博。
// 其图片信息放在 pic_ids 和 pic_infos 两个字段中。pic_ids 为字符串列表，而 pic_infos 则是嵌套之 object。
// 这两个字段，不管是外层还是嵌套的 retweeted_status 中，都可能存在。
// 发布微博时就上传图片，pic_ids/pic_infos 出现在外层字段中；而转发图片微博，pic_ids/pic_infos 出现在嵌套的 retweeted_status 中。
// 而是在转发的时候加上一张图片，则该图片仅仅被视为一个 url 而已。其信息不会出现在 pic_ids/pic_infos 中。
//
// 2)视频微博。
// 视频的短链是作为微博内容的一部分，放到 text/text_raw 字段中的。
// 短链的目的 url 形如 https://video.weibo.com/show?fid=1034:4723662272790630 (media.info.h5_url 字段也有此)
// 视频的其他元信息，则放在 page_info 中。page_info 中有如下子字段:
// * page_id: 字符串
// * object_type: 值为 video (还有可能为 article)
// * page_pic，值为视频封面图片之 url
// * media_info.duration 为视频长度，单位是秒
// 视频短链，其在 url_struct 中，会有 page_id 字段。根据 page_id 找到对应的 page_info，并根据 page_info.object_type 确定其类型。
//
// 需要注意的是，page_info 只会存在于外层中。嵌套的 retweeted_status 中是不存在此字段的。
// 当我们转发一条视频微博的时候，视频信息也只存在于外层的 page_info 之中。retweeted_status 不存在 page_info 字段。
//
// 另外，对于图片与视频微博，我们发现，
// 对于图片微博，图片信息是放在微博内容(text/text_raw)之外的；
// 而对于视频微博，视频短链是放在微博的内容(text/text_raw)之内的。视频的其他元信息，放在了单独的字段里。
// 这里的不同，猜测是因为微博自刚诞生时，其设计便已支持图片，而视频则是微博流行很久之后才支持的。
// 如果将视频 url 放在微博的内容之外，那么旧版的应用，就完全没有办法消费视频了。将视频短链放到微博内容中，用户至少可以直接打开链接到新的页面消费。
//
// 3)短链。
// 微博内容(text/text_raw)中，链接都是短链，形如 http://t.cn/A6JMdBYy。
// 而链接的目的 url 及其他元信息，则在 url_struct 中。url_struct 为 object 数组。
// 数组元素的字段包括：
// * long_url 即是短链对应的目的 url，「长链」。
// * url_type 即 url 的类型，0 表示一般的 url。而 39 则表示为视频或者文章，会有对应的 page_id 和 page_info。
// (本项目中，只将微博分为文本、图片、视频三类。文章，不作为单独的微博类型)
//
//
// 其他的一些问题：
// * 微博内容很长时，isLongText 字段为 true，此时 text/text_raw 中只包含一部分的内容。
// * 微博某些情况下不可见，返回的字段也不同，包括：
//   * 被作者删除
//   * 因作者设置原因而不可见(比如设置「仅半年内可见」的情况)
//   * 监管原因被夹
//   需要依次列出来，并正确处理。

#[derive(Clone, Debug, Deserialize)]
pub struct RawPost {
    id: i64,
    mblogid: String,
    user: User,
    text_raw: String,
    #[serde(default, rename(deserialize = "isLongText"))]
    is_long_text: bool,

    #[serde(default)]
    pic_ids: Vec<String>,
    #[serde(default)]
    pic_infos: HashMap<String, PicInfo>,

    #[serde(default, rename(deserialize = "url_struct"))]
    url_structs: Vec<UrlStruct>,
    page_info: Option<PageInfo>,

    #[serde(deserialize_with = "parse_weibo_datetime")]
    pub created_at: DateTime<FixedOffset>,

    #[serde(rename(deserialize = "retweeted_status"))]
    retweeted_post: Option<RawRetweetedPost>,
}

fn parse_weibo_datetime<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: Deserializer<'de>,
{
    // 微博 API 返回的时间形如: Sun Jan 09 11:50:55 +0800 2022
    let format = "%a %b %d %H:%M:%S %z %Y";
    let buf = String::deserialize(deserializer)?;
    let datetime = DateTime::parse_from_str(&buf, format).map_err(serde::de::Error::custom)?;
    Ok(datetime)
}

#[derive(Clone, Debug, Deserialize)]
struct RawRetweetedPost {
    id: i64,
    mblogid: String,
    user: Option<User>,
    text_raw: String,
    #[serde(default, rename(deserialize = "isLongText"))]
    is_long_text: bool,

    #[serde(default)]
    pic_ids: Vec<String>,
    #[serde(default)]
    pic_infos: HashMap<String, PicInfo>,

    #[serde(deserialize_with = "parse_weibo_datetime")]
    pub created_at: DateTime<FixedOffset>,
}

#[derive(Clone, Debug, Deserialize)]
struct PicInfo {
    original: PicInfoEntry,
}

#[derive(Clone, Debug, Deserialize)]
struct PicInfoEntry {
    url: String,
    // width: i32,
    // height: i32,
}

#[derive(Clone, Debug, Deserialize)]
struct UrlStruct {
    short_url: String,
    #[serde(default)]
    long_url: String,
    // url_type: i32,
    page_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct PageInfo {
    page_id: String,
    #[serde(default)]
    object_type: String,
    #[serde(default)]
    page_pic: String,
    media_info: Option<Value>,
}

impl PageInfo {
    fn get_video_duration_secs(&self) -> Option<u64> {
        if self.object_type == "video" {
            if let Some(media_info) = &self.media_info {
                if let Some(map) = media_info.as_object() {
                    if let Some(val) = map.get("duration") {
                        return val.as_u64();
                    }
                }
            }
        }
        None
    }
}

impl RawPost {
    pub fn normalize(self) -> Post {
        normalize_raw_post(self)
    }
}

fn normalize_raw_post(mut raw_post: RawPost) -> Post {
    let video_entry = replace_short_urls(
        &mut raw_post.text_raw,
        &raw_post.url_structs,
        &raw_post.page_info,
    );

    let mut post = Post {
        id: raw_post.id,
        mblogid: raw_post.mblogid,
        user: raw_post.user,
        text_raw: raw_post.text_raw,
        is_long_text: raw_post.is_long_text,
        media_asset: MediaAsset::None,
        created_at: raw_post.created_at,
        retweeted_post: None,
    };

    if let Some(video_entry) = video_entry {
        post.media_asset = MediaAsset::Video(video_entry);
    } else if let Some(picture_asset) = collect_picture_asset(raw_post.pic_ids, raw_post.pic_infos)
    {
        post.media_asset = picture_asset;
    }

    if let Some(retweeted_post) = raw_post.retweeted_post {
        post.retweeted_post = Some(Box::new(normalize_raw_retweeted_post(
            retweeted_post,
            &raw_post.url_structs,
            &raw_post.page_info,
        )));
    }
    post
}

fn normalize_raw_retweeted_post(
    mut retweeted_post: RawRetweetedPost,
    url_structs: &[UrlStruct],
    page_info: &Option<PageInfo>,
) -> Post {
    let video_entry = replace_short_urls(&mut retweeted_post.text_raw, url_structs, page_info);

    let mut post = Post {
        id: retweeted_post.id,
        mblogid: retweeted_post.mblogid,
        user: if retweeted_post.user.is_some() { retweeted_post.user.unwrap() } else { User::default() },
        text_raw: retweeted_post.text_raw,
        is_long_text: retweeted_post.is_long_text,
        media_asset: MediaAsset::None,
        created_at: retweeted_post.created_at,
        retweeted_post: None,
    };

    if let Some(video_entry) = video_entry {
        post.media_asset = MediaAsset::Video(video_entry);
    } else if let Some(picture_asset) =
        collect_picture_asset(retweeted_post.pic_ids, retweeted_post.pic_infos)
    {
        post.media_asset = picture_asset;
    }

    post
}

fn replace_short_urls(
    text_raw: &mut String,
    url_structs: &[UrlStruct],
    page_info: &Option<PageInfo>,
) -> Option<VideoEntry> {
    let mut url_mapping = HashMap::new();
    for url_struct in url_structs {
        url_mapping.insert(url_struct.short_url.clone(), url_struct);
    }

    let p = Regex::new(r"http://t.cn/\w{8}").unwrap();
    let mut res = None;
    while let Some(m) = p.find(text_raw) {
        // so ugly...
        if let Some(url_struct) = url_mapping.get(m.as_str()) {
            if let Some(ref page_id) = url_struct.page_id {
                if let Some(page_info) = page_info {
                    if &page_info.page_id == page_id {
                        if let Some(duration_secs) = page_info.get_video_duration_secs() {
                            res = Some(VideoEntry {
                                url: url_struct.long_url.clone(),
                                duration_secs: duration_secs as u32,
                                cover_picture_url: page_info.page_pic.clone(),
                            });
                        }
                    }
                }
            }

            let replaced = p.replace(text_raw, url_struct.long_url.clone());
            if replaced != text_raw.as_str() {
                *text_raw = replaced.into_owned();
            } else {
                break;
            }
        } else {
            break;
        }
    }
    res
}

fn collect_picture_asset(
    pic_ids: Vec<String>,
    mut pic_infos: HashMap<String, PicInfo>,
) -> Option<MediaAsset> {
    let mut picture_urls = vec![];
    for pic_id in &pic_ids {
        if let Some(entry) = pic_infos.remove(pic_id) {
            picture_urls.push(entry.original.url);
        }
    }
    if !picture_urls.is_empty() {
        Some(MediaAsset::Pictures(picture_urls))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::offset::FixedOffset;
    use chrono::{DateTime, TimeZone};

    #[test]
    fn test_parse_weibo_datetime() -> Result<(), anyhow::Error> {
        let expected_datetime = FixedOffset::east(8 * 3600)
            .ymd(2022, 1, 9)
            .and_hms(9, 40, 29);

        let txt = "Sun Jan 09 09:40:29 +0800 2022";
        let format = "%a %b %d %H:%M:%S %z %Y";
        let dt = DateTime::parse_from_str(txt, format)?;
        assert_eq!(dt, expected_datetime);
        Ok(())
    }

    #[test]
    fn test_parse_text_weibo_post() -> Result<(), anyhow::Error> {
        let raw: RawPost = serde_json::from_str(include_str!("../../data/text.json"))?;
        let post = raw.normalize();
        assert_eq!(post.media_asset, MediaAsset::None);
        assert_eq!(
            post.created_at,
            FixedOffset::east(8 * 3600)
                .ymd(2022, 1, 9)
                .and_hms(11, 50, 55)
        );
        assert!(!post.is_retweet());
        Ok(())
    }

    #[test]
    fn test_parse_retweeted_text_weibo_post() -> Result<(), anyhow::Error> {
        let raw: RawPost = serde_json::from_str(include_str!("../../data/text_retweet.json"))?;
        let post = raw.normalize();
        assert_eq!(post.media_asset, MediaAsset::None);
        assert!(post.is_retweet());
        Ok(())
    }

    #[test]
    fn test_parse_picture_weibo_post() -> Result<(), anyhow::Error> {
        let raw: RawPost = serde_json::from_str(include_str!("../../data/picture.json"))?;
        let post = raw.normalize();

        // 此微博中的图片数量为 18，但 pic_infos 中只列出了前 9 张的信息
        // 看来，当图片数量超过 9 张，就会如此。
        assert_eq!(
            post.media_asset,
            MediaAsset::Pictures(vec![
                "https://wx4.sinaimg.cn/orj1080/663aa05aly1gy4bls1n7zj20zk0npgo2.jpg".to_string(),
                "https://wx4.sinaimg.cn/orj1080/663aa05aly1gy4bls1c4mg20k00j61kx.gif".to_string(),
                "https://wx3.sinaimg.cn/orj1080/663aa05aly1gy4bls1f3zj20hr0chgm4.jpg".to_string(),
                "https://wx3.sinaimg.cn/orj1080/663aa05aly1gy4bls3v67j20hr0chgmf.jpg".to_string(),
                "https://wx3.sinaimg.cn/orj1080/663aa05aly1gy4bls1zq7j20hr0chglu.jpg".to_string(),
                "https://wx4.sinaimg.cn/orj1080/663aa05aly1gy4bls4r0kj20hr0chdgc.jpg".to_string(),
                "https://wx3.sinaimg.cn/orj1080/663aa05aly1gy4bls69eij20hr0chq30.jpg".to_string(),
                "https://wx1.sinaimg.cn/orj1080/663aa05aly1gy4bls9eu2j20hr0ch74c.jpg".to_string(),
                "https://wx1.sinaimg.cn/orj1080/663aa05aly1gy4blsbr3sj20hr0chgmh.jpg".to_string(),
            ])
        );
        assert!(!post.is_retweet());
        Ok(())
    }

    #[test]
    fn test_parse_retweeted_picture_weibo_post() -> Result<(), anyhow::Error> {
        let raw: RawPost = serde_json::from_str(include_str!("../../data/picture_retweet.json"))?;
        let post = raw.normalize();
        assert!(post.is_retweet());
        assert_eq!(
            post.retweeted_post.unwrap().media_asset,
            MediaAsset::Pictures(vec![
                "https://wx4.sinaimg.cn/orj1080/663aa05aly1gy4bls1n7zj20zk0npgo2.jpg".to_string(),
                "https://wx4.sinaimg.cn/orj1080/663aa05aly1gy4bls1c4mg20k00j61kx.gif".to_string(),
                "https://wx3.sinaimg.cn/orj1080/663aa05aly1gy4bls1f3zj20hr0chgm4.jpg".to_string(),
                "https://wx3.sinaimg.cn/orj1080/663aa05aly1gy4bls3v67j20hr0chgmf.jpg".to_string(),
                "https://wx3.sinaimg.cn/orj1080/663aa05aly1gy4bls1zq7j20hr0chglu.jpg".to_string(),
                "https://wx4.sinaimg.cn/orj1080/663aa05aly1gy4bls4r0kj20hr0chdgc.jpg".to_string(),
                "https://wx3.sinaimg.cn/orj1080/663aa05aly1gy4bls69eij20hr0chq30.jpg".to_string(),
                "https://wx1.sinaimg.cn/orj1080/663aa05aly1gy4bls9eu2j20hr0ch74c.jpg".to_string(),
                "https://wx1.sinaimg.cn/orj1080/663aa05aly1gy4blsbr3sj20hr0chgmh.jpg".to_string(),
            ])
        );
        Ok(())
    }

    #[test]
    fn test_parse_video_weibo_post() -> Result<(), anyhow::Error> {
        let raw: RawPost = serde_json::from_str(include_str!("../../data/video.json"))?;
        let post = raw.normalize();
        assert!(!post.is_retweet());
        assert_eq!(
            post.media_asset,
            MediaAsset::Video(VideoEntry {
                url: "https://video.weibo.com/show?fid=1034:4723662272790630".to_string(),
                duration_secs: 69,
                cover_picture_url:
                    "http://wx4.sinaimg.cn/orj480/7f071607ly1gy769tzli4j20k00zkq4g.jpg".to_string(),
            })
        );
        Ok(())
    }

    #[test]
    fn test_parse_retweeted_video_weibo_post() -> Result<(), anyhow::Error> {
        let raw: RawPost = serde_json::from_str(include_str!("../../data/video_retweet.json"))?;
        let post = raw.normalize();

        assert!(post.is_retweet());
        assert_eq!(
            post.retweeted_post.unwrap().media_asset,
            MediaAsset::Video(VideoEntry {
                url: "https://video.weibo.com/show?fid=1034:4725900089163796".to_string(),
                duration_secs: 63,
                cover_picture_url:
                    "http://wx2.sinaimg.cn/orj480/537f5932gy1gyeb8psxhvj21hc0u0422.jpg".to_string(),
            })
        );
        Ok(())
    }
}
