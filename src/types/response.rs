use std::convert::TryInto;

use anyhow::*;
use uriparse::URIReference;
use crate::types::{ResponseHeader, Body, Mime, Document};
use crate::util::Cowy;
use crate::GEMINI_MIME;

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

    pub fn document(document: Document) -> Self {
        Self::success_with_body(&GEMINI_MIME, document)
    }

    pub fn input(prompt: impl Cowy<str>) -> Result<Self> {
        let header = ResponseHeader::input(prompt)?;
        Ok(Self::new(header))
    }

    pub fn input_lossy(prompt: impl Cowy<str>) -> Self {
        let header = ResponseHeader::input_lossy(prompt);
        Self::new(header)
    }

    pub fn success(mime: &Mime) -> Self {
        let header = ResponseHeader::success(&mime);
        Self::new(header)
    }

    pub fn redirect_temporary_lossy<'a>(location: impl TryInto<URIReference<'a>>) -> Self {
        let header = ResponseHeader::redirect_temporary_lossy(location);
        Self::new(header)
    }

    /// Create a successful response with a preconfigured body
    ///
    /// This is equivilent to:
    ///
    /// ```norun
    /// Response::success(mime)
    ///     .with_body(body)
    /// ```
    pub fn success_with_body(mime: &Mime, body: impl Into<Body>) -> Self {
        Self::success(mime)
            .with_body(body)
    }

    pub fn server_error(reason: impl Cowy<str>) -> Result<Self>  {
        let header = ResponseHeader::server_error(reason)?;
        Ok(Self::new(header))
    }

    pub fn not_found() -> Self {
        let header = ResponseHeader::not_found();
        Self::new(header)
    }

    pub fn bad_request_lossy(reason: impl Cowy<str>) -> Self {
        let header = ResponseHeader::bad_request_lossy(reason);
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
