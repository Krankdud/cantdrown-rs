use async_trait::async_trait;
use serde_json::Value;
use songbird::input::{
    error::{Error, Result},
    restartable::Restart,
    Codec, Container, Input, Metadata, Reader, Restartable,
};
use std::{
    io::{BufRead, BufReader, Read},
    process::{Command, Stdio},
    time::Duration,
};
use tokio::{process::Command as TokioCommand, task};

pub async fn restartable_ytdl_normalized<P: AsRef<str> + Send + Clone + Sync + 'static>(
    uri: P,
    lazy: bool,
) -> Result<Restartable> {
    Restartable::new(YtdlNormalized { uri }, lazy).await
}

struct YtdlNormalized<P>
where
    P: AsRef<str> + Send + Sync,
{
    uri: P,
}

#[async_trait]
impl<P> Restart for YtdlNormalized<P>
where
    P: AsRef<str> + Send + Sync,
{
    async fn call_restart(&mut self, time: Option<Duration>) -> Result<Input> {
        if let Some(time) = time {
            let ts = format!("{:.3}", time.as_secs_f64());

            ytdl(self.uri.as_ref(), &["-ss", &ts]).await
        } else {
            ytdl(self.uri.as_ref(), &[]).await
        }
    }

    async fn lazy_init(&mut self) -> Result<(Option<Metadata>, Codec, Container)> {
        _ytdl_metadata(self.uri.as_ref())
            .await
            .map(|m| (Some(m), Codec::FloatPcm, Container::Raw))
    }
}

async fn ytdl(uri: &str, pre_args: &[&str]) -> Result<Input> {
    let ytdl_args = [
        "--print-json",
        "-f",
        "webm[abr>0]/bestaudio/best",
        "-R",
        "infinite",
        "--no-playlist",
        "--ignore-config",
        "--no-warnings",
        uri,
        "-o",
        "-",
    ];

    let ffmpeg_args = [
        "-f",
        "s16le",
        "-ac",
        "2",
        "-ar",
        "48000",
        "-acodec",
        "pcm_f32le",
        "-af",
        "loudnorm=I=-16:LRA=11:TP=-1.5",
        "-",
    ];

    let mut youtube_dl = Command::new("youtube-dl")
        .args(&ytdl_args)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    // This rigmarole is required due to the inner synchronous reading context.
    let stderr = youtube_dl.stderr.take();
    let (returned_stderr, value) = task::spawn_blocking(move || {
        let mut s = stderr.unwrap();
        let out: Result<Value> = {
            let mut o_vec = vec![];
            let mut serde_read = BufReader::new(s.by_ref());
            // Newline...
            if let Ok(len) = serde_read.read_until(0xA, &mut o_vec) {
                serde_json::from_slice(&o_vec[..len]).map_err(|err| Error::Json {
                    error: err,
                    parsed_text: std::str::from_utf8(&o_vec).unwrap_or_default().to_string(),
                })
            } else {
                Result::Err(Error::Metadata)
            }
        };

        (s, out)
    })
    .await
    .map_err(|_| Error::Metadata)?;

    youtube_dl.stderr = Some(returned_stderr);

    let taken_stdout = youtube_dl.stdout.take().ok_or(Error::Stdout)?;

    let ffmpeg = Command::new("ffmpeg")
        .args(pre_args)
        .arg("-i")
        .arg("-")
        .args(&ffmpeg_args)
        .stdin(taken_stdout)
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;

    let metadata = Metadata::from_ytdl_output(value?);

    // trace!("ytdl metadata {:?}", metadata);

    Ok(Input::new(
        true,
        Reader::from(vec![youtube_dl, ffmpeg]),
        Codec::FloatPcm,
        Container::Raw,
        Some(metadata),
    ))
}

async fn _ytdl_metadata(uri: &str) -> Result<Metadata> {
    // Most of these flags are likely unused, but we want identical search
    // and/or selection as the above functions.
    let ytdl_args = [
        "-j",
        "-f",
        "webm[abr>0]/bestaudio/best",
        "-R",
        "infinite",
        "--no-playlist",
        "--ignore-config",
        "--no-warnings",
        uri,
        "-o",
        "-",
    ];

    let youtube_dl_output = TokioCommand::new("youtube-dl")
        .args(&ytdl_args)
        .stdin(Stdio::null())
        .output()
        .await?;

    let o_vec = youtube_dl_output.stderr;

    let end = (&o_vec)
        .iter()
        .position(|el| *el == 0xA)
        .unwrap_or_else(|| o_vec.len());

    let value = serde_json::from_slice(&o_vec[..end]).map_err(|err| Error::Json {
        error: err,
        parsed_text: std::str::from_utf8(&o_vec).unwrap_or_default().to_string(),
    })?;

    let metadata = Metadata::from_ytdl_output(value);

    Ok(metadata)
}
