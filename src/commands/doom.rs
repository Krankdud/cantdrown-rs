use anyhow::Context;
use bytes::Buf;
use glob::glob;
use serenity::client::Context as SerenityContext;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{ffi::OsStr, fs::File};
use std::{io::copy, time::Duration};
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

    let config = crate::config::doom::get(&ctx).await;
    let iwad = match iwad.as_str() {
        "doom" => config.iwads.doom,
        "doom2" => config.iwads.doom2,
        "tnt" => config.iwads.tnt,
        "plutonia" => config.iwads.plutonia,
        _ => {
            msg.channel_id.say(&ctx.http, "Invalid IWAD").await?;
            return Ok(());
        }
    };

    if let Some(url) = url {
        let mut download_url = Some(String::from(&url));

        if url.contains("doomworld.com/idgames") {
            download_url = get_idgames_download_url(&url, &config.idgames_mirror);
        } else if url.contains("dropbox.com") {
            download_url = get_dropbox_download_url(&url);
        } else if url.contains("drive.google.com") {
            download_url = get_google_drive_download_url(&url);
        }

        if let Some(url) = download_url {
            let path = match download_zip(&url).await {
                Ok(path) => path,
                Err(_e) => {
                    // TODO: Log error
                    msg.channel_id
                        .say(&ctx.http, "Could not download the wad")
                        .await?;
                    return Ok(());
                }
            };

            let search = format!("{}/**/*.wad", path.to_str().unwrap());
            let first_wad = glob(&search)?.filter_map(Result::ok).next();
            let mut server_name = String::from(config.base_name);
            if let Some(first_wad) = first_wad {
                let name = first_wad.file_name().unwrap_or(OsStr::new("unknown"));
                server_name.push_str(&format!(" ({})", name.to_str().unwrap_or("unknown")));
            }

            let mut child = Command::new(config.executable)
                .arg("-host")
                .arg("-iwad")
                .arg(iwad)
                .arg("-file")
                .args(glob(&search)?.filter_map(Result::ok))
                .args(config.arguments.split_whitespace())
                .arg("+sv_hostname")
                .arg(&server_name)
                .spawn()?;

            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Created Zandronum server \"{}\", have fun!", &server_name),
                )
                .await?;

            tokio::time::sleep(Duration::from_secs(config.timeout)).await;

            let _ = child.kill();
        }
    }

    Ok(())
}

async fn download_zip(url: &str) -> anyhow::Result<PathBuf> {
    let res = reqwest::get(url)
        .await
        .context("Could not download zip file")?;

    let mut tmpfile: File = tempfile::tempfile()?;
    let content = res.bytes().await?;
    let hash = blake3::Hasher::new().update(&content).finalize();
    copy(&mut content.reader(), &mut tmpfile)?;

    let mut zip = zip::ZipArchive::new(tmpfile)?;

    let path = format!("./tmp/{}", hash.to_hex());
    let path = Path::new(&path);
    zip.extract(path)?;

    Ok(path.to_path_buf())
}

fn get_idgames_download_url(url: &str, mirror: &str) -> Option<String> {
    if let Some(index) = url.find("doomworld.com/idgames") {
        let index = index + "doomworld.com/idgames".len();
        let level = &url[index..];

        let mut url = String::from(mirror);
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
