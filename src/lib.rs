#[macro_use] extern crate log;

use std::{
    panic::AssertUnwindSafe,
    convert::TryFrom,
    io::BufReader,
    sync::Arc,
    time::Duration,
};
use futures::{future::BoxFuture, FutureExt};
use tokio::{
    prelude::*,
    io::{self, BufStream},
    net::{TcpStream, ToSocketAddrs},
    time::timeout,
};
use tokio::net::TcpListener;
use rustls::ClientCertVerifier;
use tokio_rustls::{rustls, TlsAcceptor};
use rustls::*;
use anyhow::*;
use lazy_static::lazy_static;

pub mod types;
pub mod util;

pub use mime;
pub use uriparse as uri;
pub use types::*;

pub const REQUEST_URI_MAX_LEN: usize = 1024;
pub const GEMINI_PORT: u16 = 1965;

type Handler = Arc<dyn Fn(Request) -> HandlerResponse + Send + Sync>;
type HandlerResponse = BoxFuture<'static, Result<Response>>;

#[derive(Clone)]
pub struct Server {
    tls_acceptor: TlsAcceptor,
    listener: Arc<TcpListener>,
    handler: Handler,
    timeout: Duration,
    complex_timeout: Option<Duration>,
}

impl Server {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> Builder<A> {
        Builder::bind(addr)
    }

    async fn serve(self) -> Result<()> {
        loop {
            let (stream, _addr) = self.listener.accept().await
                .context("Failed to accept client")?;
            let this = self.clone();

            tokio::spawn(async move {
                if let Err(err) = this.serve_client(stream).await {
                    error!("{:?}", err);
                }
            });
        }
    }

    async fn serve_client(self, stream: TcpStream) -> Result<()> {
        let fut_accept_request = async {
            let stream = self.tls_acceptor.accept(stream).await
                .context("Failed to establish TLS session")?;
            let mut stream = BufStream::new(stream);

            let request = receive_request(&mut stream).await
                .context("Failed to receive request")?;

            Result::<_, anyhow::Error>::Ok((request, stream))
        };

        // Use a timeout for interacting with the client
        let fut_accept_request = timeout(self.timeout, fut_accept_request);
        let (mut request, mut stream) = fut_accept_request.await
            .context("Client timed out while waiting for response")??;

        debug!("Client requested: {}", request.uri());

        // Identify the client certificate from the tls stream.  This is the first
        // certificate in the certificate chain.
        let client_cert = stream.get_ref()
            .get_ref()
            .1
            .get_peer_certificates()
            .and_then(|mut v| if v.is_empty() {None} else {Some(v.remove(0))});

        request.set_cert(client_cert);

        let handler = (self.handler)(request);
        let handler = AssertUnwindSafe(handler);

        let response = handler.catch_unwind().await
            .unwrap_or_else(|_| Response::server_error(""))
            .or_else(|err| {
                error!("Handler failed: {:?}", err);
                Response::server_error("")
            })
            .context("Request handler failed")?;

            self.send_response(response, &mut stream).await
                .context("Failed to send response")?;

        Ok(())
    }

    async fn send_response(&self, mut response: Response, stream: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
        let maybe_body = response.take_body();
        let header = response.header();

        // Okay, I know this method looks really complicated, but I promise it's not.
        // There's really only three things this method does:
        //
        // * Send the response header
        // * Send the response body
        // * Flush the stream
        //
        // All the other code is doing one of two things.  Either it's
        //
        // * code to add and handle timeouts (that's what all the async blocks and calls
        //   to timeout are), or
        // * logic to decide whether to use the special case timeout handling (seperate
        //   timeouts for the header and the body) vs the normal timeout handling (header,
        //   body, and flush all as one timeout)
        //
        // The split between the two cases happens at this very first if block.
        // Everything in this deep chain of if's and if-let's is for the special case.  If
        // any one of the ifs fails, the code after the big if block is run, and that's
        // the normal case.
        //
        // Hope this helps! Emi <3

        if header.status == Status::SUCCESS && maybe_body.is_some() {
            // aaaa let me have if-let chaining ;_;
            if let "text/plain"|"text/gemini" = header.meta.as_str() {
                if let Some(cplx_timeout) = self.complex_timeout {


        ////////////// Use the special case timeout override /////////////////////////////

                    // Send the header & flush
                    let fut_send_header = async {
                        send_response_header(response.header(), stream).await
                            .context("Failed to write response header")?;

                        stream.flush()
                            .await
                            .context("Failed to flush response header")
                    };
                    timeout(self.timeout, fut_send_header)
                        .await
                        .context("Timed out while sending response header")??;

                    // Send the body & flush
                    let fut_send_body = async {
                        send_response_body(maybe_body.unwrap(), stream).await
                            .context("Failed to write response body")?;

                        stream.flush()
                            .await
                            .context("Failed to flush response body")
                    };
                    timeout(cplx_timeout, fut_send_body)
                        .await
                        .context("Timed out while sending response body")??;

                    return Ok(())
                }
            }
        }


        ///////////// Use the normal timeout /////////////////////////////////////////////

        let fut_send_response = async {
            send_response_header(response.header(), stream).await
                .context("Failed to write response header")?;

            if let Some(body) = maybe_body {
                send_response_body(body, stream).await
                    .context("Failed to write response body")?;
            }

            stream.flush()
                .await
                .context("Failed to flush response data")
        };
        timeout(self.timeout, fut_send_response)
            .await
            .context("Timed out while sending response data")??;

        Ok(())

        //////////////////////////////////////////////////////////////////////////////////
    }
}

pub struct Builder<A> {
    addr: A,
    timeout: Duration,
    complex_body_timeout_override: Option<Duration>,
}

impl<A: ToSocketAddrs> Builder<A> {
    fn bind(addr: A) -> Self {
        Self {
            addr,
            timeout: Duration::from_secs(1),
            complex_body_timeout_override: Some(Duration::from_secs(30)),
        }
    }

    /// Set the timeout on incoming requests
    ///
    /// Note that this timeout is applied twice, once for the delivery of the request, and
    /// once for sending the client's response.  This means that for a 1 second timeout,
    /// the client will have 1 second to complete the TLS handshake and deliver a request
    /// header, then your API will have as much time as it needs to handle the request,
    /// before the client has another second to receive the response.
    ///
    /// If you would like a timeout for your code itself, please use
    /// [`tokio::time::Timeout`] to implement it internally.
    ///
    /// **The default timeout is 1 second.**  As somewhat of a workaround for
    /// shortcomings of the specification, this timeout, and any timeout set using this
    /// method, is overridden in special cases, specifically for MIME types outside of
    /// `text/plain` and `text/gemini`, to be 30 seconds.  If you would like to change or
    /// prevent this, please see
    /// [`override_complex_body_timeout`](Self::override_complex_body_timeout()).
    pub fn set_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Override the timeout for complex body types
    ///
    /// Many clients choose to handle body types which cannot be displayed by prompting
    /// the user if they would like to download or open the request body.  However, since
    /// this prompt occurs in the middle of receiving a request, often the connection
    /// times out before the end user is able to respond to the prompt.
    ///
    /// As a workaround, it is possible to set an override on the request timeout in
    /// specific conditions:
    ///
    /// 1. **Only override the timeout for receiving the body of the request.**  This will
    ///    not override the timeout on sending the request header, nor on receiving the
    ///    response header.
    /// 2. **Only override the timeout for successful responses.**  The only bodies which
    ///    have bodies are successful ones.  In all other cases, there's no body to
    ///    timeout for
    /// 3. **Only override the timeout for complex body types.**  Almost all clients are
    ///    able to display `text/plain` and `text/gemini` responses, and will not prompt
    ///    the user for these response types.  This means that there is no reason to
    ///    expect a client to have a human-length response time for these MIME types.
    ///    Because of this, responses of this type will not be overridden.
    ///
    /// This method is used to override the timeout for responses meeting these specific
    /// criteria.  All other stages of the connection will use the timeout specified in
    /// [`set_timeout()`](Self::set_timeout()).
    ///
    /// If this is set to [`None`], then the client will have the default amount of time
    /// to both receive the header and the body.  If this is set to [`Some`], the client
    /// will have the default amount of time to recieve the header, and an *additional*
    /// alotment of time to recieve the body.
    ///
    /// The default timeout for this is 30 seconds.
    pub fn override_complex_body_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.complex_body_timeout_override = timeout;
        self
    }

    pub async fn serve<F>(self, handler: F) -> Result<()>
    where
        F: Fn(Request) -> HandlerResponse + Send + Sync + 'static,
    {
        let config = tls_config()
            .context("Failed to create TLS config")?;

        let listener = TcpListener::bind(self.addr).await
            .context("Failed to create socket")?;

        let server = Server {
            tls_acceptor: TlsAcceptor::from(config),
            listener: Arc::new(listener),
            handler: Arc::new(handler),
            timeout: self.timeout,
            complex_timeout: self.complex_body_timeout_override,
        };

        server.serve().await
    }
}

async fn receive_request(stream: &mut (impl AsyncBufRead + Unpin)) -> Result<Request> {
    let limit = REQUEST_URI_MAX_LEN + "\r\n".len();
    let mut stream = stream.take(limit as u64);
    let mut uri = Vec::new();

    stream.read_until(b'\n', &mut uri).await?;

    if !uri.ends_with(b"\r\n") {
        if uri.len() < REQUEST_URI_MAX_LEN {
            bail!("Request header not terminated with CRLF")
        } else {
            bail!("Request URI too long")
        }
    }

    // Strip CRLF
    uri.pop();
    uri.pop();

    let uri = URIReference::try_from(&*uri)
        .context("Request URI is invalid")?
        .into_owned();
    let request = Request::from_uri(uri)
        .context("Failed to create request from URI")?;

    Ok(request)
}

async fn send_response_header(header: &ResponseHeader, stream: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
    let header = format!(
        "{status} {meta}\r\n",
        status = header.status.code(),
        meta = header.meta.as_str(),
    );

    stream.write_all(header.as_bytes()).await?;

    Ok(())
}

async fn send_response_body(body: Body, stream: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
    match body {
        Body::Bytes(bytes) => stream.write_all(&bytes).await?,
        Body::Reader(mut reader) => { io::copy(&mut reader, stream).await?; },
    }

    Ok(())
}

fn tls_config() -> Result<Arc<ServerConfig>> {
    let mut config = ServerConfig::new(AllowAnonOrSelfsignedClient::new());

    let cert_chain = load_cert_chain()
        .context("Failed to load TLS certificate")?;
    let key = load_key()
        .context("Failed to load TLS key")?;
    config.set_single_cert(cert_chain, key)
        .context("Failed to use loaded TLS certificate")?;

    Ok(config.into())
}

fn load_cert_chain() -> Result<Vec<Certificate>> {
    let cert_path = "cert/cert.pem";
    let certs = std::fs::File::open(cert_path)
        .with_context(|| format!("Failed to open `{}`", cert_path))?;
    let mut certs = BufReader::new(certs);
    let certs = rustls::internal::pemfile::certs(&mut certs)
        .map_err(|_| anyhow!("failed to load certs `{}`", cert_path))?;

    Ok(certs)
}

fn load_key() -> Result<PrivateKey> {
    let key_path = "cert/key.pem";
    let keys = std::fs::File::open(key_path)
        .with_context(|| format!("Failed to open `{}`", key_path))?;
    let mut keys = BufReader::new(keys);
    let mut keys = rustls::internal::pemfile::pkcs8_private_keys(&mut keys)
        .map_err(|_| anyhow!("failed to load key `{}`", key_path))?;

    ensure!(!keys.is_empty(), "no key found");

    let key = keys.swap_remove(0);

    Ok(key)
}

/// Mime for Gemini documents
pub const GEMINI_MIME_STR: &str = "text/gemini";

lazy_static! {
    /// Mime for Gemini documents ("text/gemini")
    pub static ref GEMINI_MIME: Mime = GEMINI_MIME_STR.parse().expect("northstar BUG");
}

#[deprecated(note = "Use `GEMINI_MIME` instead", since = "0.3.0")]
pub fn gemini_mime() -> Result<Mime> {
    Ok(GEMINI_MIME.clone())
}

/// A client cert verifier that accepts all connections
///
/// Unfortunately, rustls doesn't provide a ClientCertVerifier that accepts self-signed
/// certificates, so we need to implement this ourselves.
struct AllowAnonOrSelfsignedClient { }
impl AllowAnonOrSelfsignedClient {

    /// Create a new verifier
    fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

}

impl ClientCertVerifier for AllowAnonOrSelfsignedClient {

    fn client_auth_root_subjects(
        &self,
        _: Option<&webpki::DNSName>
    ) -> Option<DistinguishedNames> {
        Some(Vec::new())
    }

    fn client_auth_mandatory(&self, _sni: Option<&webpki::DNSName>) -> Option<bool> {
        Some(false)
    }

    fn verify_client_cert(
        &self,
        _: &[Certificate],
        _: Option<&webpki::DNSName>
    ) -> Result<ClientCertVerified, TLSError> {
        Ok(ClientCertVerified::assertion())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gemini_mime_parses() {
        let _: &Mime = &GEMINI_MIME;
    }
}
