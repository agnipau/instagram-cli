mod api;

use api::{HighlightReel, Post, UserInfos, Users};
use clap::{App, AppSettings, Arg, SubCommand};

#[tokio::main]
async fn main() {
    let matches = App::new("Instagram-CLI")
        .version("0.1")
        .author("Matteo Guarda <matteoguarda@tutanota.com>")
        .about("A CLI for instagram")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("search")
                .about("Searches for a profile")
                .arg(
                    Arg::with_name("QUERY")
                        .required(true)
                        .help("Query to search for"),
                ),
        )
        .subcommand(
            SubCommand::with_name("stories")
                .about("Logs infos about stories posted by an user")
                .arg(
                    Arg::with_name("USERNAME")
                        .required(true)
                        .help("Username of the user who posted the stories"),
                ),
        )
        .subcommand(
            SubCommand::with_name("dl-highlight")
                .about("Logs infos specifically about an highlight reel given its ID")
                .arg(
                    Arg::with_name("HIGHLIGHT_REEL_ID")
                        .required(true)
                        .help("ID of the highlight reel"),
                ),
        )
        .subcommand(
            SubCommand::with_name("show-highlights")
                .about("Logs infos about highlight reels posted by an user")
                .arg(
                    Arg::with_name("USERNAME")
                        .required(true)
                        .help("Username of the user who posted the highlight reels"),
                ),
        )
        .subcommand(
            SubCommand::with_name("infos")
                .about("Shows infos about an user")
                .arg(
                    Arg::with_name("USERNAME")
                        .required(true)
                        .help("Username of the user"),
                ),
        )
        .subcommand(
            SubCommand::with_name("dl-profile")
                .about("Logs direct urls to medias of an Instagram profile")
                .arg(
                    Arg::with_name("USERNAME")
                        .required(true)
                        .help("Username of the profile to scrape"),
                )
                .arg(
                    Arg::with_name("cursor")
                        .short("c")
                        .value_name("CURSOR")
                        .help("Cursor, used to start fetching from a certain point on"),
                )
                .arg(
                    Arg::with_name("iterations")
                        .short("i")
                        .value_name("NUM_ITERATIONS")
                        .help("Number of iterations to do"),
                ),
        )
        .subcommand(
            SubCommand::with_name("dl-post")
                .about("Logs the direct url of a post")
                .arg(
                    Arg::with_name("POST_ID")
                        .required(true)
                        .help("ID of the post"),
                ),
        )
        .get_matches();

    if let Some(matches) = matches.subcommand_matches("search") {
        let query = matches.value_of("QUERY").unwrap();
        if let Ok(users) = Users::new(query).await {
            println!("{}", serde_json::to_string(&users).unwrap());
        }
        return;
    }

    if let Some(matches) = matches.subcommand_matches("stories") {
        let username = matches.value_of("USERNAME").unwrap();
        if let Ok(user_data) = UserInfos::new(username).await {
            if let Ok(stories) = user_data.fetch_stories().await {
                println!("{}", serde_json::to_string(&stories).unwrap());
            }
        }
        return;
    }

    if let Some(matches) = matches.subcommand_matches("dl-highlight") {
        let hi_reel_id = matches.value_of("HIGHLIGHT_REEL_ID").unwrap();
        if let Ok(highlights) = HighlightReel::fetch_highlights(hi_reel_id).await {
            println!("{}", serde_json::to_string(&highlights).unwrap());
        }
        return;
    }

    if let Some(matches) = matches.subcommand_matches("show-highlights") {
        let username = matches.value_of("USERNAME").unwrap();
        if let Ok(user_data) = UserInfos::new(username).await {
            if let Ok(highlight_reels) = user_data.fetch_highlight_reels().await {
                println!("{}", serde_json::to_string(&highlight_reels).unwrap());
            }
        }
        return;
    }

    if let Some(matches) = matches.subcommand_matches("infos") {
        let username = matches.value_of("USERNAME").unwrap();
        if let Ok(user_data) = UserInfos::new(username).await {
            println!("{}", serde_json::to_string(&user_data).unwrap());
        }
        return;
    }

    if let Some(matches) = matches.subcommand_matches("dl-profile") {
        let username = matches.value_of("USERNAME").unwrap();
        let mut cursor = matches.value_of("cursor").map(|x| x.to_string());
        let iterations = matches
            .value_of("iterations")
            .map(|x| x.parse().expect("iterations is not a valid u64"))
            .unwrap_or(std::u64::MAX);

        // NOTE: An username is not case sensitive
        if let Ok(user_data) = UserInfos::new(username).await {
            let mut i = 0;
            while i != iterations {
                i += 1;
                if let Ok(posts) = user_data.fetch_posts(cursor.as_deref()).await {
                    println!("{}", serde_json::to_string(&posts).unwrap());
                    cursor = posts.cursor.map(|x| x.to_string());
                } else {
                    break;
                }

                if cursor.is_none() {
                    break;
                }
            }
        }
        return;
    }

    if let Some(matches) = matches.subcommand_matches("dl-post") {
        let post_id = matches.value_of("POST_ID").unwrap();
        // NOTE: An url of a post is case sensitive
        if let Ok(post) = Post::from_id(post_id).await {
            println!("{}", serde_json::to_string(&post).unwrap());
        }
    }
}
