use anyhow::{anyhow, ensure, Context, Result};
use rustls::sign::CertifiedKey;
use rustls::{
    client::danger::HandshakeSignatureValid,
    pki_types::{CertificateDer, PrivateKeyDer, UnixTime},
    server::danger::{ClientCertVerified, ClientCertVerifier},
    server::ResolvesServerCertUsingSni,
};
use rustls::{DigitallySignedStruct, DistinguishedName, Error, ServerConfig, SignatureScheme};
use std::{collections::HashMap, io::BufReader, path::PathBuf, sync::Arc};

pub fn tls_config(cert_path: &PathBuf, key_path: &PathBuf) -> Result<Arc<ServerConfig>> {
    let cert_chain = load_cert_chain(cert_path).context("Failed to load TLS certificate")?;
    let key_der = load_key(key_path).context("Failed to load TLS key")?;

    let config = ServerConfig::builder()
        .with_client_cert_verifier(AllowAnonOrSelfsignedClient::new())
        .with_single_cert(cert_chain, key_der.clone_key())
        .unwrap();

    Ok(Arc::new(config))
}

pub fn tls_sni_config(
    host_cert_key: &HashMap<String, (PathBuf, PathBuf)>,
) -> Result<Arc<ServerConfig>> {
    let mut resolver = ResolvesServerCertUsingSni::new();

    let config =
        ServerConfig::builder().with_client_cert_verifier(AllowAnonOrSelfsignedClient::new());

    for (hostname, cert_key) in host_cert_key {
        let cert_chain = load_cert_chain(&cert_key.0)
            .context("Failed to load TLS certificate")?
            .clone();
        let key_der = load_key(&cert_key.1).context("Failed to load TLS key")?;

        let certified_key = CertifiedKey::from_der(cert_chain, key_der, config.crypto_provider())?;
        resolver.add(hostname, certified_key)?
    }

    let config = config.with_cert_resolver(Arc::new(resolver));

    Ok(Arc::new(config))
}

fn load_cert_chain(cert_path: &PathBuf) -> Result<Vec<CertificateDer<'static>>> {
    let certs = std::fs::File::open(cert_path)
        .with_context(|| format!("Failed to open `{:?}`", cert_path))?;
    let mut certs = BufReader::new(certs);
    let certs = rustls_pemfile::certs(&mut certs)
        .collect::<Result<_, std::io::Error>>()
        .map_err(|_| anyhow!("failed to load certs `{:?}`", cert_path))?;

    Ok(certs)
}

fn load_key(key_path: &PathBuf) -> Result<PrivateKeyDer<'static>> {
    let keys = std::fs::File::open(key_path)
        .with_context(|| format!("Failed to open `{:?}`", key_path))?;
    let mut keys = BufReader::new(keys);
    let mut keys: Vec<_> = rustls_pemfile::pkcs8_private_keys(&mut keys)
        .collect::<Result<_, std::io::Error>>()
        .map_err(|_| anyhow!("failed to load key `{:?}`", key_path))?;

    ensure!(!keys.is_empty(), "no key found");

    let key = keys.swap_remove(0);

    Ok(PrivateKeyDer::Pkcs8(key))
}

/// A client cert verifier that accepts all connections
///
/// Unfortunately, rustls doesn't provide a ClientCertVerifier that accepts self-signed
/// certificates, so we need to implement this ourselves.
#[derive(Debug)]
struct AllowAnonOrSelfsignedClient {}
impl AllowAnonOrSelfsignedClient {
    /// Create a new verifier
    fn new() -> Arc<Self> {
        Arc::new(Self {})
    }
}

impl ClientCertVerifier for AllowAnonOrSelfsignedClient {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }

    fn verify_client_cert(
        &self,
        _: &CertificateDer,
        _: &[CertificateDer],
        _: UnixTime,
    ) -> Result<ClientCertVerified, Error> {
        Ok(ClientCertVerified::assertion())
    }

    fn client_auth_mandatory(&self) -> bool {
        false
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _mess: &[u8],
        _cert: &CertificateDer,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}
