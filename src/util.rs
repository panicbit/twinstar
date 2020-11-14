use std::path::Path;
use mime::Mime;
use percent_encoding::utf8_percent_encode;
use anyhow::*;
use tokio::{
    fs::{self, File},
    io,
};
use crate::{GEMINI_MIME, GEMINI_MIME_STR, Response};
use itertools::Itertools;

pub async fn serve_file<P: AsRef<Path>>(path: P, mime: &Mime) -> Result<Response> {
    let path = path.as_ref();

    let file = match File::open(path).await {
        Ok(file) => file,
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => return Ok(Response::not_found()),
            _ => return Err(err.into()),
        }
    };

    Ok(Response::success(&mime).with_body(file))
}

pub async fn serve_dir<D: AsRef<Path>, P: AsRef<Path>>(dir: D, virtual_path: &[P]) -> Result<Response> {
    debug!("Dir: {}", dir.as_ref().display());
    let dir = dir.as_ref().canonicalize()
        .context("Failed to canonicalize directory")?;
    let mut path = dir.to_path_buf();

    for segment in virtual_path {
        path.push(segment);
    }

    let path = path.canonicalize()
        .context("Failed to canonicalize path")?;

    if !path.starts_with(&dir) {
        return Ok(Response::not_found());
    }

    if !path.is_dir() {
        let mime = guess_mime_from_path(&path);
        return serve_file(path, &mime).await;
    }

    serve_dir_listing(path, virtual_path).await
}

async fn serve_dir_listing<P: AsRef<Path>, B: AsRef<Path>>(path: P, virtual_path: &[B]) -> Result<Response> {
    use std::fmt::Write;

    let mut dir = match fs::read_dir(path).await {
        Ok(dir) => dir,
        Err(err) => match err.kind() {
            io::ErrorKind::NotFound => return Ok(Response::not_found()),
            _ => return Err(err.into()),
        }
    };

    let breadcrumbs = virtual_path.iter().map(|segment| segment.as_ref().display()).join("/");
    let mut listing = String::new();
    
    writeln!(listing, "# Index of /{}", breadcrumbs)?;
    writeln!(listing)?;

    if virtual_path.get(0).map(<_>::as_ref) != Some(Path::new("")) {
        writeln!(listing, "=> .. 📁 ../")?;
    }

    while let Some(entry) = dir.next_entry().await.context("Failed to list directory")? {
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        let is_dir = entry.file_type().await
            .with_context(|| format!("Failed to get file type of `{}`", entry.path().display()))?
            .is_dir();

        writeln!(
            listing,
            "=> {link}{trailing_slash} {icon} {name}{trailing_slash}",
            icon = if is_dir { '📁' } else { '📄' },
            link = utf8_percent_encode(&file_name, percent_encoding::NON_ALPHANUMERIC),
            trailing_slash = if is_dir { "/" } else { "" },
            name = file_name,
        )?;
    }

    Ok(Response::success(&GEMINI_MIME).with_body(listing))
}

pub fn guess_mime_from_path<P: AsRef<Path>>(path: P) -> Mime {
    let path = path.as_ref();
    let extension = path.extension().and_then(|s| s.to_str());
    let mime = match extension {
        Some(extension) => match extension {
            "gemini" => GEMINI_MIME_STR,
            "txt" => "text/plain",
            "jpeg" | "jpg" | "jpe" => "image/jpeg",
            "png" => "image/png",
            _ => "application/octet-stream",
        },
        None => "application/octet-stream",
    };
    
    mime.parse::<Mime>().unwrap_or(mime::APPLICATION_OCTET_STREAM)
}
