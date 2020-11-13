#[macro_use] extern crate log;

use std::{panic::AssertUnwindSafe, convert::TryFrom, io::BufReader, sync::Arc};
use futures::{future::BoxFuture, FutureExt};
use mime::Mime;
use tokio::{
    prelude::*,
    io::{self, BufStream},
    net::{TcpStream, ToSocketAddrs},
};
use tokio::net::TcpListener;
use rustls::ClientCertVerifier;
use tokio_rustls::{rustls, TlsAcceptor};
use rustls::*;
use anyhow::*;
use uri::URIReference;

pub mod types;
pub mod util;

pub use mime;
pub use uriparse as uri;
pub use types::*;
pub use rustls::Certificate;

pub const REQUEST_URI_MAX_LEN: usize = 1024;
pub const GEMINI_PORT: u16 = 1965;

type Handler = Arc<dyn Fn(Request) -> HandlerResponse + Send + Sync>;
type HandlerResponse = BoxFuture<'static, Result<Response>>;

#[derive(Clone)]
pub struct Server {
    tls_acceptor: TlsAcceptor,
    listener: Arc<TcpListener>,
    handler: Handler,
}

impl Server {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> Builder<A> {
        Builder::bind(addr)
    }

    async fn serve(self) -> Result<()> {
        loop {
            let (stream, _addr) = self.listener.accept().await?;
            let this = self.clone();

            tokio::spawn(async move {
                if let Err(err) = this.serve_client(stream).await {
                    error!("{}", err);
                }
            });
        }
    }

    async fn serve_client(self, stream: TcpStream) -> Result<()> {
        let stream = self.tls_acceptor.accept(stream).await?;
        let mut stream = BufStream::new(stream);

        let mut request = receive_request(&mut stream).await?;
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
                error!("Handler: {}", err);
                Response::server_error("")
            })?;

        send_response(response, &mut stream).await?;

        stream.flush().await?;

        Ok(())
    }
}

pub struct Builder<A> {
    addr: A,
}

impl<A: ToSocketAddrs> Builder<A> {
    fn bind(addr: A) -> Self {
        Self { addr }
    }

    pub async fn serve<F>(self, handler: F) -> Result<()>
    where
        F: Fn(Request) -> HandlerResponse + Send + Sync + 'static,
    {
        let config = tls_config()?;

        let server = Server {
            tls_acceptor: TlsAcceptor::from(config),
            listener: Arc::new(TcpListener::bind(self.addr).await?),
            handler: Arc::new(handler),
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

    let uri = URIReference::try_from(&*uri)?.into_owned();
    let request = Request::from_uri(uri)?;

    Ok(request)
}

async fn send_response(mut response: Response, stream: &mut (impl AsyncWrite + Unpin)) -> Result<()> {
    send_response_header(response.header(), stream).await?;

    if let Some(body) = response.take_body() {
        send_response_body(body, stream).await?;
    }

    Ok(())
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

    let cert_chain = load_cert_chain()?;
    let key = load_key()?;
    config.set_single_cert(cert_chain, key)?;

    Ok(config.into())
}

fn load_cert_chain() -> Result<Vec<Certificate>> {
    let certs = std::fs::File::open("cert/cert.pem")?;
    let mut certs = BufReader::new(certs);
    let certs = rustls::internal::pemfile::certs(&mut certs)
        .map_err(|_| anyhow!("failed to load certs"))?;

    Ok(certs)
}

fn load_key() -> Result<PrivateKey> {
    let mut keys = BufReader::new(std::fs::File::open("cert/key.pem")?);
    let mut keys = rustls::internal::pemfile::pkcs8_private_keys(&mut keys)
        .map_err(|_| anyhow!("failed to load key"))?;

    ensure!(!keys.is_empty(), "no key found");

    let key = keys.swap_remove(0);

    Ok(key)
}

const GEMINI_MIME_STR: &str = "text/gemini";

pub fn gemini_mime() -> Result<Mime> {
    let mime = GEMINI_MIME_STR.parse()?;
    Ok(mime)
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
