use anyhow::Context;
use bytes::Buf;
use serenity::client::Context as SerenityContext;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use std::fs::File;
use std::io::copy;
use url::Url;

#[command]
async fn host(ctx: &SerenityContext, msg: &Message, mut args: Args) -> CommandResult {
    let iwad = match args.single::<String>() {
        Ok(iwad) => iwad,
        Err(_) => {
            msg.channel_id
                .say(&ctx.http, "Must provide the IWAD")
                .await?;
            return Ok(());
        }
    };
    let url = match args.single::<String>() {
        Ok(url) => Some(url),
        Err(_) => {
            msg.channel_id.say(&ctx.http, "Must provide a url").await?;
            return Ok(());
        }
    };

    if let Some(url) = url {
        let mut download_url = Some(String::from(&url));

        if url.contains("doomworld.com/idgames") {
            download_url = get_idgames_download_url(&url);
        } else if url.contains("dropbox.com") {
            download_url = get_dropbox_download_url(&url);
        } else if url.contains("drive.google.com") {
            download_url = get_google_drive_download_url(&url);
        }

        if let Some(url) = download_url {
            match download_zip(&url).await {
                Ok(_) => {}
                Err(e) => {
                    // TODO: Log error
                    msg.channel_id
                        .say(&ctx.http, "Could not download the wad")
                        .await?;
                    return Ok(());
                }
            };
        }
    }

    Ok(())
}

async fn download_zip(url: &str) -> anyhow::Result<()> {
    let res = reqwest::get(url)
        .await
        .context("Could not download zip file")?;

    println!("Status: {}", res.status());

    let mut tmpfile: File = tempfile::tempfile()?;
    let content = res.bytes().await?;
    copy(&mut content.reader(), &mut tmpfile)?;

    let zip = zip::ZipArchive::new(tmpfile)?;
    for name in zip.file_names() {
        println!("{}", name);
    }

    Ok(())
}

fn get_idgames_download_url(url: &str) -> Option<String> {
    if let Some(index) = url.find("doomworld.com/idgames") {
        let index = index + "doomworld.com/idgames".len();
        let level = &url[index..];

        // TODO: Make mirror configurable
        let mut url = String::from("http://www.gamers.org/pub/idgames");
        url.push_str(level);
        url.push_str(".zip");

        return Some(url);
    }

    None
}

fn get_dropbox_download_url(url: &str) -> Option<String> {
    let mut url = match Url::parse(url) {
        Ok(url) => url,
        Err(_) => {
            // TODO: Log error
            return None;
        }
    };
    // All that needs to be done to get a downloadable Dropbox link is
    // to set the query string.
    url.set_query(Some("raw=1"));
    Some(url.to_string())
}

fn get_google_drive_download_url(url: &str) -> Option<String> {
    let url = match Url::parse(url) {
        Ok(url) => url,
        Err(_) => {
            // TODO: Log error
            return None;
        }
    };

    // Google Drive links have the path /file/d/<id>/view
    // We need to extract the id from the path so we can create a url that we can
    // download the file from.
    let mut path_segments = url.path_segments().unwrap();
    path_segments.next();
    path_segments.next();
    let id = match path_segments.next() {
        Some(id) => id,
        None => {
            // TODO: Log error
            return None;
        }
    };

    let mut url = String::from("https://drive.google.com/uc?export=download&id=");
    url.push_str(id);

    Some(url)
}
