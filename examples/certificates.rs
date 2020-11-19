use anyhow::*;
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use log::LevelFilter;
use tokio::sync::RwLock;
use northstar::{Certificate, GEMINI_MIME, GEMINI_PORT, Request, Response, Server};
use std::collections::HashMap;
use std::sync::Arc;

// Workaround for Certificates not being hashable
type CertBytes = Vec<u8>;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_module("northstar", LevelFilter::Debug)
        .init();

    let users = Arc::<RwLock::<HashMap<CertBytes, String>>>::default();

    Server::bind(("0.0.0.0", GEMINI_PORT))
        .serve(move|req| handle_request(users.clone(), req))
        .await
}

/// An ultra-simple demonstration of simple authentication.
///
/// If the user attempts to connect, they will be prompted to create a client certificate.
/// Once they've made one, they'll be given the opportunity to create an account by
/// selecting a username.  They'll then get a message confirming their account creation.
/// Any time this user visits the site in the future, they'll get a personalized welcome
/// message.
fn handle_request(users: Arc<RwLock<HashMap<CertBytes, String>>>, request: Request) -> BoxFuture<'static, Result<Response>> {
    async move {
        if let Some(Certificate(cert_bytes)) = request.certificate() {
            // The user provided a certificate
            let users_read = users.read().await;
            if let Some(user) = users_read.get(cert_bytes) {
                // The user has already registered
                Ok(
                    Response::success_with_body(
                        &GEMINI_MIME,
                        format!("Welcome {}!", user)
                    )
                )
            } else {
                // The user still needs to register
                drop(users_read);
                if let Some(query_part) = request.uri().query() {
                    // The user provided some input (a username request)
                    let username = query_part.as_str();
                    let mut users_write = users.write().await;
                    users_write.insert(cert_bytes.clone(), username.to_owned());
                    Ok(
                        Response::success_with_body(
                            &GEMINI_MIME,
                            format!(
                                "Your account has been created {}!  Welcome!",
                                username
                            )
                        )
                    )
                } else {
                    // The user didn't provide input, and should be prompted
                    Response::input("What username would you like?")
                }
            }
        } else {
            // The user didn't provide a certificate
            Ok(Response::client_certificate_required())
        }
    }.boxed()
}
