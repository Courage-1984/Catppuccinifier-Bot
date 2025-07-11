#![allow(deprecated)]
// src/main.rs

use serenity::prelude::*;
use dotenv::dotenv;
use tracing_subscriber;
use tracing::info;
use serenity::framework::standard::{StandardFramework, CommandResult, Args, macros::{command, group}};
use serenity::model::channel::Message;

mod commands;
mod image_processing;
mod palette;
mod utils;

#[group]
#[commands(cat)]
struct General;

#[command]
async fn cat(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    use tracing::{info, warn, error};
    use crate::utils;
    use crate::palette;
    use crate::image_processing;
    use image::ImageReader;
    let arg_string = args.rest();
    info!(content = %msg.content, user = %msg.author.name, "Received !cat command (framework)");
    let parts: Vec<&str> = arg_string.split_whitespace().collect();

    // Help command
    if parts.get(0).map_or(false, |&p| p == "-h" || p == "--help" || p == "help") {
        if let Err(why) = crate::commands::send_help_message(ctx, msg.channel_id).await {
            error!(?why, "Error sending help message");
        }
        return Ok(());
    }

    // Palette command
    if parts.get(0) == Some(&"palette") {
        if let Some(&flavor) = parts.get(1) {
            if flavor == "all" {
                let palette_img = palette::generate_all_palettes_preview();
                let mut output_buffer = std::io::Cursor::new(Vec::new());
                if let Err(_e) = palette_img.write_to(&mut output_buffer, image::ImageFormat::Png) {
                    let _ = msg.channel_id.say(&ctx, "Failed to generate palette preview.").await;
                    return Ok(());
                }
                let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), "catppuccin_palettes_all.png");
                let message_content = "**All Catppuccin Color Palettes**\nFrom left to right: Latte, Frappe, Macchiato, Mocha";
                let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                let _ = msg.channel_id.send_files(&ctx, vec![attachment_data], message_builder).await;
                return Ok(());
            } else if let Some(flavor_enum) = utils::parse_flavor(flavor) {
                let palette_img = palette::generate_palette_preview(flavor_enum);
                let mut output_buffer = std::io::Cursor::new(Vec::new());
                if let Err(_e) = palette_img.write_to(&mut output_buffer, image::ImageFormat::Png) {
                    let _ = msg.channel_id.say(&ctx, "Failed to generate palette preview.").await;
                    return Ok(());
                }
                let filename = format!("catppuccin_palette_{}.png", flavor_enum.to_string().to_lowercase());
                let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename);
                let message_content = format!("**Catppuccin {} Color Palette**", flavor_enum.to_string().to_uppercase());
                let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                let _ = msg.channel_id.send_files(&ctx, vec![attachment_data], message_builder).await;
                return Ok(());
            }
        }
        let _ = msg.channel_id.say(&ctx, "Invalid palette command. Use `!cat palette [flavor]` or `!cat palette all`").await;
        return Ok(());
    }

    // Parse command arguments for flavor, algorithm, quality, format, etc.
    let mut selected_flavor = utils::parse_flavor("latte").unwrap(); // Default flavor
    let mut selected_algorithm = "shepards-method"; // Default algorithm
    let mut _batch_mode = false; // TODO: Implement batch processing

    if arg_string.split_whitespace().any(|arg| arg == "-f") {
        selected_algorithm = "nearest-neighbor";
        let _ = msg.channel_id.say(&ctx, "⚡ Fast mode enabled! Your image will be processed using the fastest settings (nearest-neighbor algorithm).").await;
    }

    if parts.len() > 0 {
        if let Some(flavor) = utils::parse_flavor(parts[0]) {
            selected_flavor = flavor;
        } else if let Some(algorithm) = utils::parse_algorithm(parts[0]) {
            selected_algorithm = algorithm;
        }
    }

    // TODO: Implement all subcommands (compare, stats, batch, all flavors, etc.)
    // For now, focus on image processing for a single image attachment
    let attachment = msg.attachments.iter().find(|a| a.width.is_some() && a.height.is_some());
    if let Some(attachment) = attachment {
        info!(filename = %attachment.filename, url = %attachment.url, "Processing image attachment");
        // Download the image
        let response = reqwest::get(&attachment.url).await;
        if let Ok(resp) = response {
            let bytes = resp.bytes().await;
            if let Ok(image_bytes) = bytes {
                let img_reader = ImageReader::new(std::io::Cursor::new(image_bytes)).with_guessed_format();
                if let Ok(reader) = img_reader {
                    if let Ok(img) = reader.decode() {
                        // Process the image using the selected flavor and algorithm
                        let processed_img = image_processing::process_image_with_palette(&img, selected_flavor, selected_algorithm);
                        let mut output_buffer = std::io::Cursor::new(Vec::new());
                        if let Err(e) = processed_img.write_to(&mut output_buffer, image::ImageFormat::Png) {
                            error!(?e, "Failed to write processed image");
                            let _ = msg.channel_id.say(&ctx, "Failed to process image.").await;
                            return Ok(());
                        }
                        let filename = format!("catppuccinified_{}.png", selected_flavor.to_string().to_lowercase());
                        let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename);
                        let message_content = format!("**Catppuccinified with {}**", selected_flavor.to_string());
                        let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                        let _ = msg.channel_id.send_files(&ctx, vec![attachment_data], message_builder).await;
                        return Ok(());
                    } else {
                        error!("Failed to decode image");
                        let _ = msg.channel_id.say(&ctx, "Failed to decode image.").await;
                        return Ok(());
                    }
                } else {
                    error!("Failed to create image reader");
                    let _ = msg.channel_id.say(&ctx, "Failed to read image.").await;
                    return Ok(());
                }
            } else {
                error!("Failed to download image bytes");
                let _ = msg.channel_id.say(&ctx, "Failed to download image.").await;
                return Ok(());
            }
        } else {
            error!("Failed to fetch image from URL");
            let _ = msg.channel_id.say(&ctx, "Failed to fetch image from URL.").await;
            return Ok(());
        }
    } else {
        warn!("No image attachment found");
        let _ = msg.channel_id.say(&ctx, "Please attach an image to process.").await;
        return Ok(());
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenv().ok();
    info!("Starting Catppuccinifier Bot...");
    let token = std::env::var("DISCORD_BOT_TOKEN")
        .expect("Expected a Discord bot token in the environment variable DISCORD_BOT_TOKEN. Make sure you have a .env file with DISCORD_BOT_TOKEN=YOUR_TOKEN_HERE");
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::DIRECT_MESSAGES;
    let framework = StandardFramework::new();
    framework.configure(serenity::framework::standard::Configuration::new().prefix("!cat"));
    let framework = framework.group(&GENERAL_GROUP);
    let mut client = Client::builder(&token, intents)
        .framework(framework)
        .event_handler(commands::Handler)
        .await
        .expect("Error creating client");
    if let Err(why) = client.start().await {
        info!(?why, "Client error");
    }
}