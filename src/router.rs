use ic_http_certification::{HttpRequest, HttpResponse};
use oc_bots_sdk_canister::{HttpMethod::POST, HttpRouter};
use std::sync::LazyLock;

// Declare modules this router uses
mod commands;
mod definition;

static ROUTER: LazyLock<HttpRouter> = LazyLock::new(|| {
    HttpRouter::default()
        // Standard endpoint for OC Bots SDK commands
        .route("/execute_command", POST, commands::execute)
        // Serves the bot's definition (metadata like name, commands)
        .fallback(definition::get)
});

// Main request handler function called by lib.rs http_request functions
pub async fn handle(request: HttpRequest, query: bool) -> HttpResponse {
    ROUTER.handle(request, query).await
}