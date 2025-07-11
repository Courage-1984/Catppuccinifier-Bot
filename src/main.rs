#![allow(deprecated)]
// src/main.rs

use serenity::prelude::*;
use dotenv::dotenv;
use tracing_subscriber::{fmt, EnvFilter};
use std::fs::File;
use tracing_appender::non_blocking;
use tracing::info;
use serenity::framework::standard::{StandardFramework, CommandResult, Args, macros::{command, group}};
use serenity::model::channel::Message;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::Layer;
use tracing_subscriber::util::SubscriberInitExt;
use tokio::sync::Semaphore;
use once_cell::sync::Lazy;
use regex::Regex;
use dashmap::DashMap;
use serenity::model::id::UserId;
use std::sync::Arc;
use image::GenericImageView;

static IMAGE_PROCESSING_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::const_new(2));
static CANCEL_FLAGS: Lazy<DashMap<UserId, Arc<std::sync::atomic::AtomicBool>>> = Lazy::new(DashMap::new);

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
            let _ = msg.channel_id.say(&ctx, "❌ Failed to send help message. Please try again later or contact the bot maintainer.").await;
        }
        return Ok(());
    }

    // Palette command
    if parts.get(0) == Some(&"palette") {
        if let Some(&flavor) = parts.get(1) {
            if flavor == "all" {
                let palette_img = palette::generate_all_palettes_preview();
                let mut output_buffer = std::io::Cursor::new(Vec::new());
                if let Err(e) = palette_img.write_to(&mut output_buffer, image::ImageFormat::Png) {
                    error!(?e, "Failed to generate all palettes preview");
                    let _ = msg.channel_id.say(&ctx, "❌ Failed to generate palette preview. Please try again later.").await;
                    return Ok(());
                }
                let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), "catppuccin_palettes_all.png");
                let message_content = "**All Catppuccin Color Palettes**\nFrom left to right: Latte, Frappe, Macchiato, Mocha";
                let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                if let Err(e) = msg.channel_id.send_files(&ctx, vec![attachment_data], message_builder).await {
                    error!(?e, "Failed to send all palettes preview");
                    let _ = msg.channel_id.say(&ctx, "❌ Failed to send palette preview. Please try again later.").await;
                }
                return Ok(());
            } else if let Some(flavor_enum) = utils::parse_flavor(flavor) {
                let palette_img = palette::generate_palette_preview(flavor_enum);
                let mut output_buffer = std::io::Cursor::new(Vec::new());
                if let Err(e) = palette_img.write_to(&mut output_buffer, image::ImageFormat::Png) {
                    error!(?e, "Failed to generate palette preview for flavor: {}", flavor);
                    let _ = msg.channel_id.say(&ctx, "❌ Failed to generate palette preview. Please try again later.").await;
                    return Ok(());
                }
                let filename = format!("catppuccin_palette_{}.png", flavor_enum.to_string().to_lowercase());
                let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename);
                let message_content = format!("**Catppuccin {} Color Palette**", flavor_enum.to_string().to_uppercase());
                let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                if let Err(e) = msg.channel_id.send_files(&ctx, vec![attachment_data], message_builder).await {
                    error!(?e, "Failed to send palette preview for flavor: {}", flavor);
                    let _ = msg.channel_id.say(&ctx, "❌ Failed to send palette preview. Please try again later.").await;
                }
                return Ok(());
            }
        }
        let _ = msg.channel_id.say(&ctx, "❌ Invalid palette command. Use `!cat palette [flavor]` or `!cat palette all`. Try `!cat help` for more info.").await;
        return Ok(());
    }

    // List command
    if parts.get(0).map_or(false, |&p| p == "list") {
        let flavors = ["latte", "frappe", "macchiato", "mocha"];
        let algorithms = [
            "shepards-method", "gaussian-rbf", "linear-rbf", "gaussian-sampling", "nearest-neighbor", "hald", "euclide", "mean", "std"
        ];
        let formats = ["png", "jpg", "webp", "gif", "bmp"];
        let mut message = String::from("**Available Catppuccinifier Options:**\n\n");
        message.push_str("**Flavors:**\n");
        for f in &flavors { message.push_str(&format!("- `{}`\n", f)); }
        message.push_str("\n**Algorithms:**\n");
        for a in &algorithms { message.push_str(&format!("- `{}`\n", a)); }
        message.push_str("\n**Formats:**\n");
        for fmt in &formats { message.push_str(&format!("- `{}`\n", fmt)); }
        let _ = msg.channel_id.say(&ctx, message).await;
        return Ok(());
    }

    // Check for cancel subcommand
    if parts.get(0).map_or(false, |&p| p == "cancel") {
        let user_id = msg.author.id;
        let flag = CANCEL_FLAGS.entry(user_id).or_insert_with(|| Arc::new(std::sync::atomic::AtomicBool::new(false)));
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
        let _ = msg.channel_id.say(&ctx, "🛑 Your Catppuccinify job will be cancelled if running.").await;
        return Ok(());
    }

    // Random color or palette command
    if parts.get(0).map_or(false, |&p| p == "random") {
        use rand::seq::SliceRandom;
        use catppuccin::PALETTE;
        let flavors = ["latte", "frappe", "macchiato", "mocha"];
        if parts.get(1).map_or(false, |&p| p == "palette") {
            // Random palette preview
            let flavor = flavors.choose(&mut rand::thread_rng()).unwrap();
            let flavor_enum = utils::parse_flavor(flavor).unwrap();
            let palette_img = palette::generate_palette_preview(flavor_enum);
            let mut output_buffer = std::io::Cursor::new(Vec::new());
            if let Err(_e) = palette_img.write_to(&mut output_buffer, image::ImageFormat::Png) {
                let _ = msg.channel_id.say(&ctx, "❌ Failed to generate palette preview.").await;
                return Ok(());
            }
            let filename = format!("catppuccin_palette_{}.png", flavor);
            let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename);
            let message_content = format!("**Random Catppuccin Palette: {}**", flavor.to_uppercase());
            let message_builder = serenity::builder::CreateMessage::new().content(message_content);
            let _ = msg.channel_id.send_files(&ctx, vec![attachment_data], message_builder).await;
            return Ok(());
    } else {
            // Random color
            let flavor = flavors.choose(&mut rand::thread_rng()).unwrap();
            let flavor_enum = utils::parse_flavor(flavor).unwrap();
            let colors_struct = match flavor_enum {
                catppuccin::FlavorName::Latte => &PALETTE.latte.colors,
                catppuccin::FlavorName::Frappe => &PALETTE.frappe.colors,
                catppuccin::FlavorName::Macchiato => &PALETTE.macchiato.colors,
                catppuccin::FlavorName::Mocha => &PALETTE.mocha.colors,
            };
            let color_names = [
                "rosewater", "flamingo", "pink", "mauve", "red", "maroon", "peach", "yellow", "green", "teal", "sky", "sapphire", "blue", "lavender", "text", "subtext1", "subtext0", "overlay2", "overlay1", "overlay0", "surface2", "surface1", "surface0", "base", "mantle", "crust"
            ];
            let color_name = color_names.choose(&mut rand::thread_rng()).unwrap();
            let color = match *color_name {
                "rosewater" => &colors_struct.rosewater,
                "flamingo" => &colors_struct.flamingo,
                "pink" => &colors_struct.pink,
                "mauve" => &colors_struct.mauve,
                "red" => &colors_struct.red,
                "maroon" => &colors_struct.maroon,
                "peach" => &colors_struct.peach,
                "yellow" => &colors_struct.yellow,
                "green" => &colors_struct.green,
                "teal" => &colors_struct.teal,
                "sky" => &colors_struct.sky,
                "sapphire" => &colors_struct.sapphire,
                "blue" => &colors_struct.blue,
                "lavender" => &colors_struct.lavender,
                "text" => &colors_struct.text,
                "subtext1" => &colors_struct.subtext1,
                "subtext0" => &colors_struct.subtext0,
                "overlay2" => &colors_struct.overlay2,
                "overlay1" => &colors_struct.overlay1,
                "overlay0" => &colors_struct.overlay0,
                "surface2" => &colors_struct.surface2,
                "surface1" => &colors_struct.surface1,
                "surface0" => &colors_struct.surface0,
                "base" => &colors_struct.base,
                "mantle" => &colors_struct.mantle,
                "crust" => &colors_struct.crust,
                _ => &colors_struct.base,
            };
            let hex = format!("#{:02X}{:02X}{:02X}", color.rgb.r, color.rgb.g, color.rgb.b);
            let message = format!("**Random Catppuccin Color**\nFlavor: `{}`\nColor: `{}`\nHex: `{}`\nSwatch: ` [48;2;{};{};{}m      [0m`", flavor, color_name, hex, color.rgb.r, color.rgb.g, color.rgb.b);
            let _ = msg.channel_id.say(&ctx, message).await;
            return Ok(());
        }
    }

    // Parse command arguments for flavor, algorithm, quality, format, etc.
    let mut selected_flavor = utils::parse_flavor("latte").unwrap();
    let mut selected_algorithm = "shepards-method"; // Default algorithm
    let mut batch_mode = false;
    let mut selected_format = None;

    if arg_string.split_whitespace().any(|arg| arg == "-f") {
        selected_algorithm = "nearest-neighbor";
        let _ = msg.channel_id.say(&ctx, "⚡ Fast mode enabled! Your image will be processed using the fastest settings (nearest-neighbor algorithm).").await;
    }

    if parts.len() > 0 {
        if parts[0] == "batch" {
            batch_mode = true;
        } else if let Some(flavor) = utils::parse_flavor(parts[0]) {
            selected_flavor = flavor;
        } else if let Some(algorithm) = utils::parse_algorithm(parts[0]) {
            selected_algorithm = algorithm;
        }
    }
    if msg.attachments.len() > 1 {
        batch_mode = true;
    }

    // Batch processing logic for multiple attachments
    if batch_mode && !msg.attachments.is_empty() {
        let mut processed_attachments = Vec::new();
        let mut failed_count = 0;
        for attachment in msg.attachments.iter() {
            let content_type_is_image = attachment.content_type.as_deref().map_or(false, |s| s.starts_with("image/"));
            if !content_type_is_image {
                continue;
            }
            let reqwest_client = reqwest::Client::new();
            let image_bytes = match reqwest_client.get(&attachment.url).send().await {
                Ok(response) => match response.bytes().await {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        failed_count += 1;
                        continue;
                    }
                },
                Err(_) => {
                    failed_count += 1;
                    continue;
                }
            };
            let img = match ImageReader::new(std::io::Cursor::new(image_bytes)).with_guessed_format().expect("Failed to guess image format").decode() {
                Ok(img) => img,
                Err(_) => {
                    failed_count += 1;
                    continue;
                }
            };
            let mut rgba_img = img.to_rgba8();
            let lut = image_processing::generate_catppuccin_lut(selected_flavor, selected_algorithm);
            image_processing::apply_lut_to_image(&mut rgba_img, &lut);
            let mut output_buffer = std::io::Cursor::new(Vec::new());
            let output_format = selected_format.unwrap_or(image::ImageFormat::Png);
            let dynamic_img = image::DynamicImage::ImageRgba8(rgba_img);
            if let Err(_) = dynamic_img.write_to(&mut output_buffer, output_format) {
                failed_count += 1;
                continue;
            }
            let filename = format!("catppuccinified_{}_{}.", selected_flavor.to_string().to_lowercase(), attachment.filename);
            let filename = if let Some(ext) = output_format.extensions_str().first() {
                format!("{}{}", filename, ext)
            } else {
                format!("{}png", filename)
            };
            let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename);
            processed_attachments.push(attachment_data);
        }
        if !processed_attachments.is_empty() {
            let message_content = if failed_count > 0 {
                format!("Here are your Catppuccinified images! ({} failed)", failed_count)
            } else {
                "Here are your Catppuccinified images!".to_string()
            };
            let message_builder = serenity::builder::CreateMessage::new().content(message_content);
            let _ = msg.channel_id.send_files(&ctx, processed_attachments, message_builder).await;
        } else {
            let _ = msg.channel_id.say(&ctx, "Failed to process any images. Please ensure your attachments are valid images.").await;
        }
        return Ok(());
    }

    // For now, focus on image processing for a single image attachment or image URL
    // Start typing indicator (ephemeral)
    let _typing = msg.channel_id.start_typing(&ctx.http);
    // Validate user input length
    if arg_string.len() > 300 {
        let _ = msg.channel_id.say(&ctx, "❌ Command too long. Please keep your command under 300 characters.").await;
        return Ok(());
    }
    // Check for image URL in arguments
    let url_regex = Regex::new(r"^(https?://[\w\-./%?=&]+\.(png|jpe?g|gif|bmp|webp))$").unwrap();
    let url_arg = parts.iter().find(|s| url_regex.is_match(s));
    let mut image_url: Option<&str> = None;
    if let Some(&url) = url_arg {
        if url.len() > 300 {
            let _ = msg.channel_id.say(&ctx, "❌ Image URL is too long.").await;
            return Ok(());
        }
        image_url = Some(url);
    }
    let attachment = msg.attachments.iter().find(|a| a.width.is_some() && a.height.is_some());
    let image_source = if let Some(url) = image_url {
        Some(url)
    } else if let Some(attachment) = attachment {
        Some(attachment.url.as_str())
                    } else {
        None
    };
    if let Some(image_url) = image_source {
        info!(url = %image_url, "Processing image from URL or attachment");
        // Download the image
        let response = reqwest::get(image_url).await;
        if let Ok(resp) = response {
            // Check file size limit (8 MB)
            if let Some(content_length) = resp.content_length() {
                if content_length > 8 * 1024 * 1024 {
                    let _ = msg.channel_id.say(&ctx, "❌ Image is too large. Maximum allowed size is 8 MB.").await;
                    return Ok(());
                }
            }
            let bytes = resp.bytes().await;
            if let Ok(image_bytes) = bytes {
                if image_bytes.len() > 8 * 1024 * 1024 {
                    let _ = msg.channel_id.say(&ctx, "❌ Image is too large. Maximum allowed size is 8 MB.").await;
                    return Ok(());
                }
                let img_reader = ImageReader::new(std::io::Cursor::new(&image_bytes)).with_guessed_format();
                if let Ok(reader) = img_reader {
                    let format = reader.format();
                    if let Some(image::ImageFormat::Gif) = format {
                        // Animated GIF: process all frames
                        let permit = IMAGE_PROCESSING_SEMAPHORE.acquire().await.expect("Semaphore closed");
                        let _ = msg.channel_id.say(&ctx, "🕒 Processing animated GIF (all frames)...").await;
                        let selected_flavor = selected_flavor.clone();
                        let selected_algorithm = selected_algorithm.to_string();
                        let gif_bytes = image_bytes.clone();
                        let processing_result = tokio::task::spawn_blocking(move || {
                            image_processing::process_gif_with_palette(&gif_bytes, selected_flavor, &selected_algorithm)
                        }).await;
                        drop(permit);
                        match processing_result {
                            Ok(Ok(gif_bytes)) => {
                                let filename = format!("catppuccinified_{}.gif", selected_flavor.to_string().to_lowercase());
                                let attachment_data = serenity::builder::CreateAttachment::bytes(gif_bytes, filename);
                                let message_content = format!("**Catppuccinified GIF with {}**", selected_flavor.to_string());
                                let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                                if let Err(e) = msg.channel_id.send_files(&ctx, vec![attachment_data], message_builder).await {
                                    error!(?e, "Failed to send processed GIF");
                                    let _ = msg.channel_id.say(&ctx, "❌ Failed to send processed GIF. Please try again later.").await;
                                }
                            }
                            Ok(Err(e)) => {
                                error!(?e, "Failed to process GIF");
                                let _ = msg.channel_id.say(&ctx, &format!("❌ Failed to process GIF: {e}")).await;
                            }
                                Err(e) => {
                                error!(?e, "GIF processing panicked or failed to run");
                                let _ = msg.channel_id.say(&ctx, "❌ GIF processing failed unexpectedly. Please try again or contact the bot maintainer.").await;
                            }
                        }
                        return Ok(());
                    }
                    if let Ok(img) = reader.decode() {
                        let (width, height) = img.dimensions();
                        if width > 4096 || height > 4096 {
                            let _ = msg.channel_id.say(&ctx, "❌ Image dimensions are too large. Maximum allowed is 4096x4096 pixels.").await;
                            return Ok(());
                        }
                        // Process the image using the selected flavor and algorithm
                        let permit = IMAGE_PROCESSING_SEMAPHORE.acquire().await.expect("Semaphore closed");
                        let _ = msg.channel_id.say(&ctx, "🕒 Your image is now being processed...").await;
                        let selected_flavor = selected_flavor.clone();
                        let selected_algorithm = selected_algorithm.to_string();
                        let img_clone = img.clone();
                        // Before starting processing, set up cancellation flag
                        let user_id = msg.author.id;
                        let cancel_flag = CANCEL_FLAGS.entry(user_id).or_insert_with(|| Arc::new(std::sync::atomic::AtomicBool::new(false))).clone();
                        cancel_flag.store(false, std::sync::atomic::Ordering::SeqCst);
                        let processing_result = tokio::task::spawn_blocking(move || {
                            // Periodically check for cancellation
                            for _ in 0..5 {
                                if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
                                    return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "Job cancelled by user"));
                                }
                                std::thread::sleep(std::time::Duration::from_millis(100));
                            }
                            let processed_img = image_processing::process_image_with_palette(&img_clone, selected_flavor, &selected_algorithm);
                            let mut output_buffer = std::io::Cursor::new(Vec::new());
                            match processed_img.write_to(&mut output_buffer, image::ImageFormat::Png) {
                                Ok(_) => Ok(output_buffer.into_inner()),
                                Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
                            }
                        }).await;
                        CANCEL_FLAGS.remove(&user_id);
                        drop(permit);
                        match processing_result {
                            Ok(Ok(image_bytes)) => {
                                let filename = format!("catppuccinified_{}.png", selected_flavor.to_string().to_lowercase());
                                let attachment_data = serenity::builder::CreateAttachment::bytes(image_bytes, filename);
                                let message_content = format!("**Catppuccinified with {}**", selected_flavor.to_string());
                        let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                                if let Err(e) = msg.channel_id.send_files(&ctx, vec![attachment_data], message_builder).await {
                                    error!(?e, "Failed to send processed image");
                                    let _ = msg.channel_id.say(&ctx, "❌ Failed to send processed image. Please try again later.").await;
                                }
                            }
                            Ok(Err(e)) => {
                                if e.kind() == std::io::ErrorKind::Interrupted {
                                    let _ = msg.channel_id.say(&ctx, "🛑 Your Catppuccinify job was cancelled.").await;
                                } else {
                                    error!(?e, "Failed to write processed image");
                                    let _ = msg.channel_id.say(&ctx, "❌ Failed to process image after conversion. Please try a different image or contact the bot maintainer.").await;
                                }
                            }
                            Err(e) => {
                                error!(?e, "Image processing panicked or failed to run");
                                let _ = msg.channel_id.say(&ctx, "❌ Image processing failed unexpectedly. Please try again or contact the bot maintainer.").await;
                            }
                        }
                        return Ok(());
                    }
                    error!(url = %image_url, "Failed to decode image");
                    let _ = msg.channel_id.say(&ctx, "❌ Failed to decode the image. Please ensure your image is a supported format (PNG, JPEG, etc.) and not corrupted.").await;
                    return Ok(());
                        } else {
                    error!(url = %image_url, "Failed to create image reader");
                    let _ = msg.channel_id.say(&ctx, "❌ Failed to read the image. Please try a different image or format.").await;
                    return Ok(());
                    }
                } else {
                error!(url = %image_url, "Failed to download image bytes");
                let _ = msg.channel_id.say(&ctx, "❌ Failed to download the image. Please check the URL or try re-uploading your image.").await;
                return Ok(());
                        }
                    } else {
            error!(url = %image_url, "Failed to fetch image from URL");
            let _ = msg.channel_id.say(&ctx, "❌ Failed to fetch the image from the provided URL. Please check the URL and try again.").await;
            return Ok(());
                }
            } else {
        warn!(user = %msg.author.name, "No image attachment or valid URL found");
        let _ = msg.channel_id.say(&ctx, "❌ No image attachment or valid image URL found. Please attach an image or provide a direct image URL (ending in .png, .jpg, .jpeg, .gif, .bmp, .webp).").await;
        return Ok(());
    }
    // At the end of the function, the typing indicator will be dropped automatically
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up logging: INFO to console, ERROR to file
    let file = File::create("catppuccin_bot.log")?;
    let (file_writer, _guard) = non_blocking(file);
    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stdout).with_filter(EnvFilter::new("info")))
        .with(fmt::layer().with_writer(file_writer).with_filter(EnvFilter::new("error")))
        .init();
    tracing::info!("Starting Catppuccinifier Bot...");
    dotenv().ok();
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
    Ok(())
}