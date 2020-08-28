use serde::{Deserialize, Serialize};

// Instagram sometimes trolls and changes this script's name
const QUERY_HASH_SCRIPT_URL: &str =
    "https://www.instagram.com/static/bundles/es6/ProfilePageContainer.js/76d5c065446f.js";
const HIGHLIGHT_REEL_QUERY_HASH: &str = "0d2a558130b34ee7f7d1e5a3cddc52c2";
const SESSION_ID: &str = "10109976513%3AjCh6P6OrQC9k5n%3A12";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/79.0.3945.130 Safari/537.36";
const USER_AGENT_MOBILE: &str = "Instagram 9.5.2 (iPhone7,2; iPhone OS 9_3_3; en_US; en-US; scale=2.00; 750x1334) AppleWebKit/420+";
const APP_ID: &str = "936619743392459";

fn extract_json_str(html: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(html
        .split("window._sharedData = ")
        .nth(1)
        .and_then(|x| x.split("};").nth(0))
        .ok_or(Box::<dyn std::error::Error>::from(
            "Instagram sent a page with an invalid window._sharedData",
        ))?
        .to_string()
        + "}")
}

async fn fetch_query_hash() -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(QUERY_HASH_SCRIPT_URL)
        .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
        .header(reqwest::header::ACCEPT_LANGUAGE, "en-US;q=0.8,en;q=0.7")
        .header(reqwest::header::CACHE_CONTROL, "max-age=0")
        .header("Sec-Fetch-Mode", "navigate")
        .header("Sec-Fetch-Site", "none")
        .header("Sec-Fetch-User", "?1")
        .header(reqwest::header::UPGRADE_INSECURE_REQUESTS, "1")
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .text()
        .await?;

    let parts = resp.split("},queryId:\"").collect::<Vec<_>>()[1..]
        .into_iter()
        .filter_map(|x| x.split('"').nth(0))
        .collect::<Vec<_>>();
    Ok(parts[parts.len() - 1].into())
}

#[derive(Serialize, Deserialize, Debug)]
struct User {
    #[serde(rename(deserialize = "pk"))]
    id: Option<String>,
    username: Option<String>,
    full_name: Option<String>,
    is_private: Option<bool>,
    is_verified: Option<bool>,
    #[serde(rename(deserialize = "profile_pic_url"))]
    profile_picture_url: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct Users {
    query: String,
    users: Vec<User>,
}

impl Users {
    pub async fn new(query: &str) -> Result<Users, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let resp = client
        .get(&format!(
            "https://www.instagram.com/web/search/topsearch/?context=blended&query={}&rank_token=0.37795423543635787&include_reel=true",
            query
        ))
        .header(reqwest::header::ACCEPT, "*/*")
        .header(reqwest::header::ACCEPT_LANGUAGE, "en-US;q=0.8,en;q=0.7")
        .header(reqwest::header::REFERER, "https://www.instagram.com/instagram/")
        .header("Sec-Fetch-Dest", "empty")
        .header("Sec-Fetch-Mode", "cors")
        .header("Sec-Fetch-Site", "same-origin")
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header("X-Ig-App-Id", APP_ID)
        .header("X-Ig-Www-Claim", "0")
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

        Ok(Users {
            query: query.into(),
            users: resp["users"]
                .as_array()
                .ok_or(Box::<dyn std::error::Error>::from(
                    "While parsing 'users', expected array, got something else",
                ))?
                .into_iter()
                .filter_map(|user| serde_json::from_value(user["user"].clone()).ok())
                .collect::<Vec<_>>(),
        })
    }
}

#[derive(Serialize, Debug)]
struct HighlightReelCoverImage {
    size: Option<Size>,
    url: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct HighlightReel {
    id: Option<String>,
    title: Option<String>,
    media_count: Option<u64>,
    cover_image: HighlightReelCoverImage,
}

#[derive(Serialize, Debug)]
pub struct Highlight {
    size: Size,
    media_url: Option<String>,
    taken_at_timestamp: Option<u64>,
    expiring_at_timestamp: Option<u64>,
    is_video: Option<bool>,
    has_audio: Option<bool>,
    video_duration: Option<f64>,
}

impl HighlightReel {
    pub async fn fetch_highlights(id: &str) -> Result<Vec<Highlight>, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let resp = client
            .get(&format!(
                "https://www.instagram.com/graphql/query?query_hash={}&variables=%7B%22highlight_reel_ids%22%3A%5B%22{}%22%5D%2C%22precomposed_overlay%22%3Afalse%2C%22story_viewer_fetch_count%22%3A50%7D",
                HIGHLIGHT_REEL_QUERY_HASH,
                id
            ))
            .header("x-ig-capabilities", "3w==")
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-GB,en-US;q=0.8,en;q=0.6")
            .header(reqwest::header::USER_AGENT, USER_AGENT_MOBILE)
            .header(reqwest::header::ACCEPT, "*/*")
            .header("Authority", "i.instagram.com/")
            .header(reqwest::header::COOKIE, format!("sessionid={}", SESSION_ID))
            .header(reqwest::header::HOST, "i.instagram.com")
            .header(reqwest::header::CONNECTION, "Keep-Alive")
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        Ok(resp["data"]["reels_media"][0]["items"]
            .as_array()
            .unwrap_or(&Vec::new())
            .into_iter()
            .map(|highlight| {
                let is_video = highlight["is_video"].as_bool();
                let media = if is_video.unwrap_or(false) {
                    let rsrcs = highlight["video_resources"].as_array();
                    rsrcs.map(|arr| arr[arr.len() - 1].clone())
                } else {
                    let rsrcs = highlight["display_resources"].as_array();
                    rsrcs.map(|arr| arr[arr.len() - 1].clone())
                };

                let (size, media_url) = match media {
                    Some(m) => (
                        Size {
                            width: m["config_width"].as_u64(),
                            height: m["config_height"].as_u64(),
                        },
                        m["src"].as_str().map(|x| x.to_string()),
                    ),
                    // TODO: Find a more elegant solution
                    None => (
                        Size {
                            width: None,
                            height: None,
                        },
                        None,
                    ),
                };

                Highlight {
                    size,
                    media_url,
                    taken_at_timestamp: highlight["taken_at_timestamp"].as_u64(),
                    expiring_at_timestamp: highlight["expiring_at_timestamp"].as_u64(),
                    is_video,
                    has_audio: highlight["has_audio"].as_bool(),
                    video_duration: highlight["video_duration"].as_f64(),
                }
            })
            .collect())
    }
}

#[derive(Serialize, Debug)]
pub struct Story {
    taken_at: Option<u64>,
    device_timestamp: Option<u64>,
    original_size: Size,
    has_audio: Option<bool>,
    video_duration: Option<f64>,
    expiring_at: Option<u64>,
    caption: Option<String>,
    is_video: bool,
    display_url: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct UserInfos {
    biography: Option<String>,
    external_url: Option<String>,
    external_url_link_shimmed: Option<String>,
    followers: Option<u64>,
    following: Option<u64>,
    full_name: Option<String>,
    id: Option<String>,
    is_business_account: Option<bool>,
    is_joined_recently: Option<bool>,
    business_category_name: Option<String>,
    is_private: Option<bool>,
    is_verified: Option<bool>,
    profile_picture_url: Option<String>,
    profile_picture_url_hd: Option<String>,
    username: Option<String>,
    connected_fb_page: Option<String>,
    collections_count: Option<u64>,
    saved_media_count: Option<u64>,
    videos_count: Option<u64>,
    medias_count: Option<u64>,
}

impl UserInfos {
    pub async fn new(username: &str) -> Result<UserInfos, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let resp = client
            .get(&format!("https://www.instagram.com/{}/", username))
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US;q=0.8,en;q=0.7")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "none")
            .header("Sec-Fetch-User", "?1")
            .header(reqwest::header::UPGRADE_INSECURE_REQUESTS, "1")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await?
            .text()
            .await?;

        let json_str = extract_json_str(&resp)?;
        let user = serde_json::from_str::<serde_json::Value>(&json_str)?["entry_data"]
            ["ProfilePage"][0]["graphql"]["user"]
            .clone();

        Ok(UserInfos {
            biography: user["biography"].as_str().map(|x| x.to_string()),
            external_url: user["external_url"].as_str().map(|x| x.to_string()),
            external_url_link_shimmed: user["external_url_link_shimmed"]
                .as_str()
                .map(|x| x.to_string()),
            followers: user["edge_followed_by"]["count"].as_u64(),
            following: user["edge_follow"]["count"].as_u64(),
            full_name: user["full_name"].as_str().map(|x| x.to_string()),
            id: user["id"].as_str().map(|x| x.to_string()),
            is_business_account: user["is_business_account"].as_bool(),
            is_joined_recently: user["is_joined_recently"].as_bool(),
            business_category_name: user["business_category_name"]
                .as_str()
                .map(|x| x.to_string()),
            is_private: user["is_private"].as_bool(),
            is_verified: user["is_verified"].as_bool(),
            profile_picture_url: user["profile_pic_url"].as_str().map(|x| x.to_string()),
            profile_picture_url_hd: user["profile_pic_url_hd"].as_str().map(|x| x.to_string()),
            username: user["username"].as_str().map(|x| x.to_string()),
            connected_fb_page: user["connected_fb_page"].as_str().map(|x| x.to_string()),
            collections_count: user["edge_media_collections"]["count"].as_u64(),
            saved_media_count: user["edge_saved_media"]["count"].as_u64(),
            videos_count: user["edge_felix_video_timeline"]["count"].as_u64(),
            medias_count: user["edge_owner_to_timeline_media"]["count"].as_u64(),
        })
    }

    pub async fn fetch_highlight_reels(
        &self,
    ) -> Result<Vec<HighlightReel>, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let id = self.id.clone().ok_or(Box::<dyn std::error::Error>::from(
            "id is None, can't fetch highlight reels",
        ))?;
        let resp = client
            .get(&format!(
                "https://i.instagram.com/api/v1/highlights/{}/highlights_tray/",
                id
            ))
            .header("X-Ig-Capabilities", "3w==")
            .header(
                reqwest::header::ACCEPT_LANGUAGE,
                "en-GB,en-US;q=0.8,en;q=0.6",
            )
            .header(reqwest::header::USER_AGENT, USER_AGENT_MOBILE)
            .header(reqwest::header::ACCEPT, "*/*")
            .header("Authority", "i.instagram.com/")
            .header(
                reqwest::header::COOKIE,
                &format!("sessionid={}", SESSION_ID),
            )
            .header(reqwest::header::HOST, "i.instagram.com")
            .header(reqwest::header::CONNECTION, "Keep-Alive")
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        Ok(resp["tray"]
            .as_array()
            .unwrap_or(&Vec::new())
            .into_iter()
            .filter_map(|highlight_reel| {
                let hi_reel = highlight_reel.as_object()?;

                let id = hi_reel["id"].as_str().and_then(|id| {
                    if id.contains("highlight:") {
                        id.split(":").nth(1).map(|x| x.to_string())
                    } else {
                        Some(id.to_string())
                    }
                });

                Some(HighlightReel {
                    id,
                    title: highlight_reel["title"].as_str().map(|x| x.to_string()),
                    media_count: highlight_reel["media_count"].as_u64(),
                    cover_image: HighlightReelCoverImage {
                        size: serde_json::from_value(
                            highlight_reel["cover_media"]["cropped_image_version"].clone(),
                        )
                        .ok(),
                        url: highlight_reel["cover_media"]["cropped_image_version"]["url"]
                            .as_str()
                            .map(|x| x.to_string()),
                    },
                })
            })
            .collect())
    }

    pub async fn fetch_stories(&self) -> Result<Vec<Story>, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let id = self.id.clone().ok_or(Box::<dyn std::error::Error>::from(
            "id is None, can't fetch stories",
        ))?;
        let resp = client
            .get(&format!(
                "https://i.instagram.com/api/v1/feed/user/{}/reel_media/",
                id
            ))
            .header("X-Ig-Capabilities", "3w==")
            .header(
                reqwest::header::ACCEPT_LANGUAGE,
                "en-GB,en-US;q=0.8,en;q=0.6",
            )
            .header(reqwest::header::USER_AGENT, USER_AGENT_MOBILE)
            .header(reqwest::header::ACCEPT, "*/*")
            .header("Authority", "i.instagram.com/")
            .header(
                reqwest::header::COOKIE,
                &format!("sessionid={}", SESSION_ID),
            )
            .header(reqwest::header::HOST, "i.instagram.com")
            .header(reqwest::header::CONNECTION, "Keep-Alive")
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        Ok(resp["items"]
            .as_array()
            .unwrap_or(&Vec::new())
            .into_iter()
            .filter_map(|item| {
                let i = item.as_object()?;
                let is_video = i.contains_key("video_versions");

                Some(Story {
                    taken_at: item["taken_at"].as_u64(),
                    device_timestamp: item["device_timestamp"].as_u64(),
                    original_size: Size {
                        width: item["original_width"].as_u64(),
                        height: item["original_width"].as_u64(),
                    },
                    has_audio: item["has_audio"].as_bool(),
                    video_duration: item["video_duration"].as_f64(),
                    expiring_at: item["expiring_at"].as_u64(),
                    caption: item["caption"].as_str().map(|x| x.to_string()),
                    is_video,
                    display_url: (if is_video {
                        item["video_versions"][0]["url"].clone()
                    } else {
                        item["image_versions2"]["candidates"][0]["url"].clone()
                    })
                    .as_str()
                    .map(|x| x.to_string()),
                })
            })
            .collect())
    }

    pub async fn fetch_posts(
        &self,
        cursor: Option<&str>,
    ) -> Result<Posts, Box<dyn std::error::Error>> {
        let query_hash = fetch_query_hash().await?;
        let cursor = cursor.unwrap_or("");

        let client = reqwest::Client::new();
        let id = self.id.clone().ok_or(Box::<dyn std::error::Error>::from(
            "id is None, can't fetch highlights",
        ))?;
        let resp = client
            .get(&format!(
                "https://www.instagram.com/graphql/query/?query_hash={}&variables=%7B%22id%22%3A%22{}%22%2C%22first%22%3A12%2C%22after%22%3A%22{}%22%7D",
                query_hash,
                id,
                cursor,
            ))
            .header(reqwest::header::ACCEPT, "*/*")
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US;q=0.8,en;q=0.7")
            .header(reqwest::header::REFERER, format!("https://www.instagram.com/{}/", self.username.clone().unwrap_or("instagram".into())))
            .header("Sec-Fetch-Mode", "cors")
            .header("Sec-Fetch-Site", "same-origin")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .header("X-Ig-Www-Claim", "0")
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let edge_owner_to_timeline_media =
            resp["data"]["user"]["edge_owner_to_timeline_media"].clone();

        Ok(Posts {
            cursor: edge_owner_to_timeline_media["page_info"]["end_cursor"]
                .as_str()
                .map(|x| x.to_string()),
            posts: edge_owner_to_timeline_media["edges"]
                .as_array()
                .unwrap_or(&Vec::new())
                .into_iter()
                .filter_map(|edge| Post::from_root_node(edge["node"].as_object().unwrap()).ok())
                .collect(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Size {
    width: Option<u64>,
    height: Option<u64>,
}

#[derive(Serialize, Debug)]
pub struct Post {
    id: Option<String>,
    dimensions: Option<Size>,
    is_video: Option<bool>,
    accessibility_caption: Option<String>,
    caption: Option<String>,
    short_code: Option<String>,
    comments_disabled: Option<bool>,
    taken_at_timestamp: Option<u64>,
    likes: Option<u64>,
    location: Option<String>,
    media_urls: Vec<String>,
}

impl Post {
    pub fn from_root_node(
        root_node: &serde_json::map::Map<String, serde_json::Value>,
    ) -> Result<Post, Box<dyn std::error::Error>> {
        let nodes = {
            if root_node.contains_key("edge_sidecar_to_children") {
                root_node["edge_sidecar_to_children"]["edges"]
                    .as_array()
                    .map(|edges| {
                        edges
                            .into_iter()
                            .filter_map(|x| x["node"].as_object())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or(Vec::new())
            } else {
                vec![root_node]
            }
        };

        let media_urls = nodes
            .into_iter()
            .map(|x| {
                if x.contains_key("video_url") {
                    x["video_url"].clone()
                } else {
                    x["display_url"].clone()
                }
            })
            .filter_map(|x| x.as_str().map(|z| z.to_string()))
            .collect::<Vec<_>>();

        Ok(Post {
            media_urls,
            id: root_node["id"].as_str().map(|x| x.to_string()),
            dimensions: serde_json::from_value(root_node["dimensions"].clone()).ok(),
            is_video: root_node["is_video"].as_bool(),
            accessibility_caption: if root_node.contains_key("accessibility_caption") {
                root_node["accessibility_caption"]
                    .as_str()
                    .map(|x| x.to_string())
            } else {
                None
            },
            caption: root_node["edge_media_to_caption"]["edges"][0]["node"]["text"]
                .as_str()
                .map(|x| x.to_string()),
            short_code: root_node["shortcode"].as_str().map(|x| x.to_string()),
            comments_disabled: root_node["comments_disabled"].as_bool(),
            taken_at_timestamp: root_node["taken_at_timestamp"].as_u64(),
            likes: root_node["edge_media_preview_like"]["count"].as_u64(),
            location: root_node["location"].as_str().map(|x| x.to_string()),
        })
    }

    pub async fn from_id(id: &str) -> Result<Post, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let resp = client
            .get(&format!("https://www.instagram.com/p/{}/", id))
            .header(reqwest::header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.9")
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US;q=0.8,en;q=0.7")
            .header(reqwest::header::CACHE_CONTROL, "max-age=0")
            .header(reqwest::header::REFERER, "https://www.google.com/")
            .header("Sec-Fetch-Dest", "document")
            .header("Sec-Fetch-Mode", "navigate")
            .header("Sec-Fetch-Site", "same-origin")
            .header("Sec-Fetch-Mode-User", "?1")
            .header(reqwest::header::UPGRADE_INSECURE_REQUESTS, "1")
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send().await?.text().await?;

        let json_str = extract_json_str(&resp)?;
        let parsed_data = serde_json::from_str::<serde_json::Value>(&json_str)?;

        let root_node = parsed_data["entry_data"]["PostPage"][0]["graphql"]["shortcode_media"]
            .as_object()
            .ok_or(Box::<dyn std::error::Error>::from("Masiero"))?;
        Self::from_root_node(root_node)
    }
}

#[derive(Serialize, Debug)]
pub struct Posts {
    pub cursor: Option<String>,
    posts: Vec<Post>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[test]
    fn query_hash() {
        let mut runtime = Runtime::new().expect("Failed to create tokio runtime");
        let query_hash = runtime.block_on(fetch_query_hash());
        match query_hash {
            Ok(query_hash) => println!("{}", query_hash),
            Err(_) => {
                query_hash.unwrap();
            }
        }
    }

    #[test]
    fn user_infos() {
        let mut runtime = Runtime::new().expect("Failed to create tokio runtime");
        let user_infos = runtime.block_on(UserInfos::new("instagram"));
        let user_infos = match user_infos {
            Ok(user_infos) => {
                println!("{:#?}", user_infos);
                user_infos
            }
            Err(_) => {
                user_infos.unwrap();
                return;
            }
        };

        let hi_reels = runtime.block_on(user_infos.fetch_highlight_reels());
        let hi_reels = match hi_reels {
            Ok(hi_reels) => {
                println!("{:#?}", hi_reels);
                hi_reels
            }
            Err(_) => {
                hi_reels.unwrap();
                return;
            }
        };
        let highlights = runtime.block_on(HighlightReel::fetch_highlights(
            &hi_reels[0].id.clone().unwrap(),
        ));
        match highlights {
            Ok(highlights) => println!("{:#?}", highlights),
            Err(_) => {
                highlights.unwrap();
            }
        }

        let stories = runtime.block_on(user_infos.fetch_stories());
        match stories {
            Ok(stories) => println!("{:#?}", stories),
            Err(_) => {
                stories.unwrap();
            }
        }

        let posts = runtime.block_on(user_infos.fetch_posts(None));
        match posts {
            Ok(posts) => println!("{:#?}", posts),
            Err(_) => {
                posts.unwrap();
            }
        }
    }

    #[test]
    fn users() {
        let mut runtime = Runtime::new().expect("Failed to create tokio runtime");
        let users = runtime.block_on(Users::new("fabrizio"));
        match users {
            Ok(users) => println!("{:#?}", users),
            Err(_) => {
                users.unwrap();
            }
        }
    }

    #[test]
    fn post() {
        let mut runtime = Runtime::new().expect("Failed to create tokio runtime");
        let post = runtime.block_on(Post::from_id("B_GDqtggNwa"));
        match post {
            Ok(post) => println!("{:#?}", post),
            Err(_) => {
                post.unwrap();
            }
        }
    }
}
