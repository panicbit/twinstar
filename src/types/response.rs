use anyhow::*;
use crate::types::{ResponseHeader, Body, Mime};

pub struct Response {
    header: ResponseHeader,
    body: Option<Body>,
}

impl Response {
    pub fn new(header: ResponseHeader) -> Self {
        Self {
            header,
            body: None,
        }
    }

    pub fn input(prompt: impl AsRef<str> + Into<String>) -> Result<Self> {
        let header = ResponseHeader::input(prompt)?;
        Ok(Self::new(header))
    }

    pub fn input_lossy(prompt: impl AsRef<str> + Into<String>) -> Self {
        let header = ResponseHeader::input_lossy(prompt);
        Self::new(header)
    }

    pub fn success(mime: &Mime) -> Self {
        let header = ResponseHeader::success(&mime);
        Self::new(header)
    }

    pub fn server_error(reason: impl AsRef<str> + Into<String>) -> Result<Self>  {
        let header = ResponseHeader::server_error(reason)?;
        Ok(Self::new(header))
    }

    pub fn not_found() -> Self {
        let header = ResponseHeader::not_found();
        Self::new(header)
    }

    pub fn client_certificate_required() -> Self {
        let header = ResponseHeader::client_certificate_required();
        Self::new(header)
    }

    pub fn certificate_not_authorized() -> Self {
        let header = ResponseHeader::certificate_not_authorized();
        Self::new(header)
    }

    pub fn with_body(mut self, body: impl Into<Body>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn header(&self) -> &ResponseHeader {
        &self.header
    }

    pub fn take_body(&mut self) -> Option<Body> {
        self.body.take()
    }
}
