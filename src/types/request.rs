use std::ops;
use anyhow::*;
use percent_encoding::percent_decode_str;
use uriparse::URIReference;
use rustls::Certificate;

pub struct Request {
    uri: URIReference<'static>,
    input: Option<String>,
    certificate: Option<Certificate>,
    trailing_segments: Option<Vec<String>>,
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
            trailing_segments: None,
        })
    }

    pub const fn uri(&self) -> &URIReference {
        &self.uri
    }

    #[allow(clippy::missing_const_for_fn)]
    /// All of the path segments following the route to which this request was bound.
    ///
    /// For example, if this handler was bound to the `/api` route, and a request was
    /// received to `/api/v1/endpoint`, then this value would be `["v1", "endpoint"]`.
    /// This should not be confused with [`path_segments()`](Self::path_segments()), which
    /// contains *all* of the segments, not just those trailing the route.
    ///
    /// If the trailing segments have not been set, this method will panic, but this
    /// should only be possible if you are constructing the Request yourself.  Requests
    /// to handlers registered through [`add_route`](northstar::Builder::add_route()) will
    /// always have trailing segments set.
    pub fn trailing_segments(&self) -> &Vec<String> {
        self.trailing_segments.as_ref().unwrap()
    }

    /// All of the segments in this path, percent decoded
    ///
    /// For example, for a request to `/api/v1/endpoint`, this would return `["api", "v1",
    /// "endpoint"]`, no matter what route the handler that recieved this request was
    /// bound to.  This is not to be confused with
    /// [`trailing_segments()`](Self::trailing_segments), which contains only the segments
    /// following the bound route.
    ///
    /// Additionally, unlike `trailing_segments()`, this method percent decodes the path.
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

    pub fn set_trailing(&mut self, segments: Vec<String>) {
        self.trailing_segments = Some(segments);
    }

    pub const fn certificate(&self) -> Option<&Certificate> {
        self.certificate.as_ref()
    }
}

impl ops::Deref for Request {
    type Target = URIReference<'static>;

    fn deref(&self) -> &Self::Target {
        &self.uri
    }
}
