use std::convert::TryInto;

use anyhow::{Result, Context};
use uriparse::URIReference;
use crate::Mime;
use crate::util::Cowy;
use crate::types::{Status, Meta};

#[derive(Debug,Clone)]
pub struct ResponseHeader {
    pub status: Status,
    pub meta: Meta,
}

impl ResponseHeader {
    pub fn input(prompt: impl Cowy<str>) -> Result<Self> {
        Ok(Self {
            status: Status::INPUT,
            meta: Meta::new(prompt).context("Invalid input prompt")?,
        })
    }

    pub fn input_lossy(prompt: impl Cowy<str>) -> Self {
        Self {
            status: Status::INPUT,
            meta: Meta::new_lossy(prompt),
        }
    }

    pub fn success(mime: &Mime) -> Self {
        Self {
            status: Status::SUCCESS,
            meta: Meta::new_lossy(mime.to_string()),
        }
    }

    pub fn redirect_temporary_lossy<'a>(location: impl TryInto<URIReference<'a>>) -> Self {
        let location = match location.try_into() {
            Ok(location) => location,
            Err(_) => return Self::bad_request_lossy("Invalid redirect location"),
        };

        Self {
            status: Status::REDIRECT_TEMPORARY,
            meta: Meta::new_lossy(location.to_string()),
        }
    }

    pub fn server_error(reason: impl Cowy<str>) -> Result<Self> {
        Ok(Self {
            status: Status::PERMANENT_FAILURE,
            meta: Meta::new(reason).context("Invalid server error reason")?,
        })
    }

    pub fn server_error_lossy(reason: impl Cowy<str>) -> Self {
        Self {
            status: Status::PERMANENT_FAILURE,
            meta: Meta::new_lossy(reason),
        }
    }

    pub fn not_found() -> Self {
        Self {
            status: Status::NOT_FOUND,
            meta: Meta::new_lossy("Not found"),
        }
    }

    pub fn bad_request_lossy(reason: impl Cowy<str>) -> Self {
        Self {
            status: Status::BAD_REQUEST,
            meta: Meta::new_lossy(reason),
        }
    }

    pub fn client_certificate_required() -> Self {
        Self {
            status: Status::CLIENT_CERTIFICATE_REQUIRED,
            meta: Meta::new_lossy("No certificate provided"),
        }
    }

    pub fn certificate_not_authorized() -> Self {
        Self {
            status: Status::CERTIFICATE_NOT_AUTHORIZED,
            meta: Meta::new_lossy("Your certificate is not authorized to view this content"),
        }
    }

    pub const fn status(&self) -> &Status {
        &self.status
    }

    pub const fn meta(&self) -> &Meta {
        &self.meta
    }
}
