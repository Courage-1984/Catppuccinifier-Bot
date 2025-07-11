// src/main.rs

use serenity::prelude::*;
use dotenv::dotenv;
use std::env;
use tracing_subscriber;
use tracing::info;

mod commands;
mod image_processing;
mod palette;
mod utils;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    // Load environment variables from .env file
    dotenv().ok();

    info!("Starting Catppuccinifier Bot...");

    // Load the Discord bot token from environment variables.
    let token = env::var("DISCORD_BOT_TOKEN")
        .expect("Expected a Discord bot token in the environment variable DISCORD_BOT_TOKEN. Make sure you have a .env file with DISCORD_BOT_TOKEN=YOUR_TOKEN_HERE");

    // Create a new Discord client.
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::DIRECT_MESSAGES;

    let mut client = Client::builder(&token, intents)
        .event_handler(commands::Handler)
        .await
        .expect("Error creating client");

    // Start the Discord client.
    if let Err(why) = client.start().await {
        info!(?why, "Client error");
    }
}