use anyhow::*;
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use log::LevelFilter;
use northstar::{Document, document::HeadingLevel, Request, Response, GEMINI_PORT};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_module("northstar", LevelFilter::Debug)
        .init();

    northstar::Server::bind(("localhost", GEMINI_PORT))
        .add_route("/", handle_base)
        .add_route("/route", handle_short)
        .add_route("/route/long", handle_long)
        .serve()
        .await
}

fn handle_base(_: Request) -> BoxFuture<'static, Result<Response>> {
    let doc = generate_doc("base");
    async move {
        Ok(Response::document(doc))
    }.boxed()
}

fn handle_short(_: Request) -> BoxFuture<'static, Result<Response>> {
    let doc = generate_doc("short");
    async move {
        Ok(Response::document(doc))
    }.boxed()
}

fn handle_long(_: Request) -> BoxFuture<'static, Result<Response>> {
    let doc = generate_doc("long");
    async move {
        Ok(Response::document(doc))
    }.boxed()
}

fn generate_doc(route_name: &str) -> Document {
    let mut doc = Document::new();
    doc.add_heading(HeadingLevel::H1, "Routing Demo")
       .add_text(&format!("You're currently on the {} route", route_name))
       .add_blank_line()
       .add_text("Here's some links to try:")
       .add_link_without_label("/")
       .add_link_without_label("/route")
       .add_link_without_label("/route/long")
       .add_link_without_label("/route/not_real");
    doc
}
