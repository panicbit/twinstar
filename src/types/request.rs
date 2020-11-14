use std::ops;
use anyhow::*;
use percent_encoding::percent_decode_str;
use uriparse::URIReference;
use rustls::Certificate;

pub struct Request {
    uri: URIReference<'static>,
    input: Option<String>,
    certificate: Option<Certificate>,
}

impl Request {
    pub fn from_uri(uri: URIReference<'static>) -> Result<Self> {
        Self::with_certificate(uri, None)
    }

    pub fn with_certificate(
        mut uri: URIReference<'static>,
        certificate: Option<Certificate>
    ) -> Result<Self> {
        uri.normalize();

        let input = match uri.query() {
            None => None,
            Some(query) => {
                let input = percent_decode_str(query.as_str())
                    .decode_utf8()
                    .context("Request URI query contains invalid UTF-8")?
                    .into_owned();
                Some(input)
            }
        };

        Ok(Self {
            uri,
            input,
            certificate,
        })
    }

    pub fn uri(&self) -> &URIReference {
        &self.uri
    }

    pub fn path_segments(&self) -> Vec<String> {
        self.uri()
            .path()
            .segments()
            .iter()
            .map(|segment| percent_decode_str(segment.as_str()).decode_utf8_lossy().into_owned())
            .collect::<Vec<String>>()
    }

    pub fn input(&self) -> Option<&str> {
        self.input.as_deref()
    }

    pub fn set_cert(&mut self, cert: Option<Certificate>) {
        self.certificate = cert;
    }

    pub fn certificate(&self) -> Option<&Certificate> {
        self.certificate.as_ref()
    }
}

impl ops::Deref for Request {
    type Target = URIReference<'static>;

    fn deref(&self) -> &Self::Target {
        &self.uri
    }
}
