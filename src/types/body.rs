use tokio::io::AsyncRead;
#[cfg(feature="serve_dir")]
use tokio::fs::File;

use std::borrow::Borrow;

use crate::types::Document;

pub enum Body {
    Bytes(Vec<u8>),
    Reader(Box<dyn AsyncRead + Send + Sync + Unpin>),
}

impl<D: Borrow<Document>> From<D> for Body {
    fn from(document: D) -> Self {
        Self::from(document.borrow().to_string())
    }
}

impl From<Vec<u8>> for Body {
    fn from(bytes: Vec<u8>) -> Self {
        Self::Bytes(bytes)
    }
}

impl<'a> From<&'a [u8]> for Body {
    fn from(bytes: &[u8]) -> Self {
        Self::Bytes(bytes.to_owned())
    }
}

impl From<String> for Body {
    fn from(text: String) -> Self {
        Self::Bytes(text.into_bytes())
    }
}

impl<'a> From<&'a str> for Body {
    fn from(text: &str) -> Self {
        Self::Bytes(text.to_owned().into_bytes())
    }
}

#[cfg(feature="serve_dir")]
impl From<File> for Body {
    fn from(file: File) -> Self {
        Self::Reader(Box::new(file))
    }
}
