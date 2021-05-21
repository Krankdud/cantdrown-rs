use serde_json::Value;
use tokio::process::Command;

pub async fn get_playlist_videos(url: &str) -> anyhow::Result<Vec<Option<String>>> {
    let args = ["-J", "--flat-playlist", url, "-o", "-"];

    let ytdl_output = Command::new("youtube-dl").args(&args).output().await?;
    let output_vec = ytdl_output.stderr;

    let value: Value = serde_json::from_slice(&output_vec)?;

    let empty: Vec<Value> = vec![];
    let urls: Vec<Option<String>> = value
        .as_object()
        .and_then(|m| m.get("entries"))
        .and_then(Value::as_array)
        .unwrap_or(&empty)
        .iter()
        .map(Value::as_object)
        .map(|obj| {
            obj.and_then(|m| m.get("url"))
                .and_then(Value::as_str)
                .and_then(|s| Some(String::from(s)))
        })
        .collect();

    Ok(urls)
}
