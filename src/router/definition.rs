use super::commands; // Use commands module from the same level
use oc_bots_sdk::api::definition::*;
use oc_bots_sdk_canister::{HttpRequest, HttpResponse};

// Serves the bot's definition metadata
pub async fn get(_request: HttpRequest) -> HttpResponse {
    HttpResponse::json(
        200,
        &BotDefinition {
            description: // Updated description
                "Darely Bot: Engage in fun, on-chain dare challenges! Compete, build streaks, and earn rewards.".to_string(),
            commands: commands::definitions(), // Get command list dynamically
            autonomous_config: None, // No autonomous features planned yet
        },
    )
}