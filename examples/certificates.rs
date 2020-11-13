use anyhow::*;
use futures::{future::BoxFuture, FutureExt};
use tokio::sync::RwLock;
use northstar::{Server, Request, Response, GEMINI_PORT, Certificate, gemini_mime};
use std::collections::HashMap;
use std::sync::Arc;

// Workaround for Certificates not being hashable
type CertBytes = Vec<u8>;

#[tokio::main]
async fn main() -> Result<()> {
    let users = Arc::<RwLock::<HashMap<CertBytes, String>>>::default();

    Server::bind(("0.0.0.0", GEMINI_PORT))
        .serve(move|req, cert| handle_request(users.clone(), req, cert))
        .await
}

/// An ultra-simple demonstration of simple authentication.
///
/// If the user attempts to connect, they will be prompted to create a client certificate.
/// Once they've made one, they'll be given the opportunity to create an account by
/// selecting a username.  They'll then get a message confirming their account creation.
/// Any time this user visits the site in the future, they'll get a personalized welcome
/// message.
fn handle_request(users: Arc<RwLock<HashMap<CertBytes, String>>>, request: Request, cert: Option<Certificate>) -> BoxFuture<'static, Result<Response>> {
    async move {
        if let Some(Certificate(cert_bytes)) = cert {
            // The user provided a certificate
            let users_read = users.read().await;
            if let Some(user) = users_read.get(&cert_bytes) {
                // The user has already registered
                Ok(
                    Response::success(&gemini_mime()?)?
                        .with_body(format!("Welcome {}!", user))
                )
            } else {
                // The user still needs to register
                drop(users_read);
                if let Some(query_part) = request.uri().query() {
                    // The user provided some input (a username request)
                    let username = query_part.as_str();
                    let mut users_write = users.write().await;
                    users_write.insert(cert_bytes, username.to_owned());
                    Ok(
                        Response::success(&gemini_mime()?)?
                            .with_body(format!(
                                "Your account has been created {}!  Welcome!",
                                username
                            ))
                    )
                } else {
                    // The user didn't provide input, and should be prompted
                    Response::input("What username would you like?")
                }
            }
        } else {
            // The user didn't provide a certificate
            Response::needs_certificate()
        }
    }.boxed()
}
