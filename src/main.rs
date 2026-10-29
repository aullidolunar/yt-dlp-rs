use rustop::opts;
use std::env;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Obtiene el directorio de trabajo dado un Path opcional o el actual como fallback.
fn get_working_dir(user_dir: Option<&str>) -> io::Result<PathBuf> {
    match user_dir {
        Some(dir_str) => {
            let path = Path::new(dir_str);
            if path.is_dir() {
                Ok(path.to_path_buf())
            } else {
                Err(io::Error::new(
                    ErrorKind::NotADirectory,
                    format!("'{dir_str}': is not a directory"),
                ))
            }
        }
        None => env::current_dir().or_else(|err| {
            println!("Failed to get current directory: {err}");
            Ok(PathBuf::from("."))
        }),
    }
}

fn make_yt_dlp_args(song: &gjson::Value, outputdir: &Path) -> Vec<String> {
    let artist = song.get("artist");
    let title = song.get("title");
    let url = song.get("url");
    let use_cover = song.get("use_cover").bool();

    println!("'{title}' from {artist} -> {url}");

    let mut args = vec![
        "-t".to_string(),
        "mp3".to_string(),
        "--audio-quality".to_string(),
        "128k".to_string(),
    ];

    if use_cover {
        args.push("--embed-thumbnail".to_string());
    }

    args.extend([
        "--ppa".to_string(),
        format!(r#"ffmpeg: -metadata artist="{artist}" -metadata title="{title}""#),
        "-P".to_string(),
        outputdir.display().to_string(),
        "-o".to_string(),
        format!("{artist} - {title}.%(ext)s"),
        url.to_string(),
    ]);

    args
}

fn main() -> io::Result<()> {
    let (args, _) = opts! {
        synopsis "A small Rust utility that processes a custom JSON format and automates audio downloads through yt-dlp";
        opt directory:Option<String>, desc: "output directory";
        opt file:Option<String>, desc: "input jman format json filename";
    }
    .parse_or_exit();

    let json_file = args.file.ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidInput,
            "Need valid jman format json filename",
        )
    })?;

    if !Path::new(&json_file).is_file() {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            format!("'{json_file}': file not found"),
        ));
    }

    let working_dir = get_working_dir(args.directory.as_deref())?;

    let content = std::fs::read_to_string(&json_file)?;

    if !gjson::valid(&content) {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("'{json_file}' Invalid JSON file format"),
        ));
    }

    if gjson::get(&content, "author").str() != "Joel" {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("'{json_file}': Invalid jman format JSON file. Reason: author"),
        ));
    }

    let data = gjson::get(&content, "data");
    if data.kind() != gjson::Kind::Object {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("'{json_file}': Invalid jman format JSON file. Reason: data"),
        ));
    }

    println!("Working directory: {}", working_dir.display());
    println!("Processing: {json_file}");

    data.each(|_, song| {
        let yt_dlp_args = make_yt_dlp_args(&song, &working_dir);
        let status = Command::new("yt-dlp").args(&yt_dlp_args).status();

        match status {
            Ok(st) => {
                if let Some(code) = st.code() {
                    println!("yt-dlp return code: {code}");
                }
            }
            Err(err) => eprintln!("Failed to execute yt-dlp: {err}"),
        }
        true
    });

    Ok(())
}
