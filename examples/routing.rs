use anyhow::*;
use futures_core::future::BoxFuture;
use futures_util::FutureExt;
use log::LevelFilter;
use twinstar::{Document, document::HeadingLevel, Request, Response, GEMINI_PORT};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_module("twinstar", LevelFilter::Debug)
        .init();

    twinstar::Server::bind(("localhost", GEMINI_PORT))
        .add_route("/", handle_base)
        .add_route("/route", handle_short)
        .add_route("/route/long", handle_long)
        .serve()
        .await
}

fn handle_base(req: Request) -> BoxFuture<'static, Result<Response>> {
    let doc = generate_doc("base", &req);
    async move {
        Ok(Response::document(doc))
    }.boxed()
}

fn handle_short(req: Request) -> BoxFuture<'static, Result<Response>> {
    let doc = generate_doc("short", &req);
    async move {
        Ok(Response::document(doc))
    }.boxed()
}

fn handle_long(req: Request) -> BoxFuture<'static, Result<Response>> {
    let doc = generate_doc("long", &req);
    async move {
        Ok(Response::document(doc))
    }.boxed()
}

fn generate_doc(route_name: &str, req: &Request) -> Document {
    let trailing = req.trailing_segments().join("/");
    let mut doc = Document::new();
    doc.add_heading(HeadingLevel::H1, "Routing Demo")
       .add_text(&format!("You're currently on the {} route", route_name))
       .add_text(&format!("Trailing segments: /{}", trailing))
       .add_blank_line()
       .add_text("Here's some links to try:")
       .add_link_without_label("/")
       .add_link_without_label("/route")
       .add_link_without_label("/route/long")
       .add_link_without_label("/route/not_real")
       .add_link_without_label("/rowte");
    doc
}
