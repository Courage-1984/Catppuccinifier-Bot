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
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use tokio::signal;

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
        // Start typing indicator for help command
        let _typing = msg.channel_id.start_typing(&ctx.http);
        
        // Create progress bar for help command
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {wide_msg}")
                .unwrap()
        );
        progress_bar.set_message("📚 Preparing help message...");
        progress_bar.enable_steady_tick(Duration::from_millis(100));
        
        if let Err(why) = crate::commands::send_help_message(ctx, msg.channel_id).await {
            progress_bar.finish_with_message("❌ Error sending help message");
            error!(?why, "Error sending help message");
            let _ = msg.channel_id.say(&ctx, "❌ Failed to send help message. Please try again later or contact the bot maintainer.").await;
        } else {
            progress_bar.finish_with_message("✅ Help message sent successfully!");
        }
        return Ok(());
    }

    // Palette command
    if parts.get(0) == Some(&"palette") {
        // Start typing indicator for palette command
        let _typing = msg.channel_id.start_typing(&ctx.http);
        
        // Create progress bar for palette command
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {wide_msg}")
                .unwrap()
        );
        progress_bar.set_message("🎨 Generating palette preview...");
        progress_bar.enable_steady_tick(Duration::from_millis(100));
        
        if let Some(&flavor) = parts.get(1) {
            if flavor == "all" {
                let progress_msg = "🎨 Generating all palette previews...";
                progress_bar.set_message(progress_msg);
                let palette_img = palette::generate_all_palettes_preview();
                let mut output_buffer = std::io::Cursor::new(Vec::new());
                if let Err(e) = palette_img.write_to(&mut output_buffer, image::ImageFormat::Png) {
                    progress_bar.finish_with_message("❌ Failed to generate all palettes preview");
                    error!(?e, "Failed to generate all palettes preview");
                    let _ = msg.channel_id.say(&ctx, "❌ Failed to generate palette preview. Please try again later.").await;
                    return Ok(());
                }
                let filename = utils::sanitize_filename("catppuccin_palettes_all.png", "png");
                let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename);
                let message_content = "**All Catppuccin Color Palettes**\nFrom left to right: Latte, Frappe, Macchiato, Mocha";
                let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                let progress_msg = "📤 Uploading all palette previews...";
                progress_bar.set_message(progress_msg);
                if let Err(e) = msg.channel_id.send_files(&ctx, vec![attachment_data], message_builder).await {
                    progress_bar.finish_with_message("❌ Failed to send all palettes preview");
                    error!(?e, "Failed to send all palettes preview");
                    let _ = msg.channel_id.say(&ctx, "❌ Failed to send palette preview. Please try again later.").await;
                } else {
                    progress_bar.finish_with_message("✅ All palette previews uploaded successfully!");
                }
                return Ok(());
            } else if let Some(flavor_enum) = utils::parse_flavor(flavor) {
                progress_bar.set_message("🎨 Generating palette preview...");
                let palette_img = palette::generate_palette_preview(flavor_enum);
                let mut output_buffer = std::io::Cursor::new(Vec::new());
                if let Err(e) = palette_img.write_to(&mut output_buffer, image::ImageFormat::Png) {
                    progress_bar.finish_with_message("❌ Failed to generate palette preview");
                    error!(?e, "Failed to generate palette preview for flavor: {}", flavor);
                    let _ = msg.channel_id.say(&ctx, "❌ Failed to generate palette preview. Please try again later.").await;
                    return Ok(());
                }
                let filename = utils::sanitize_filename(&format!("catppuccin_palette_{}.png", flavor_enum.to_string().to_lowercase()), "png");
                let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename);
                let message_content = format!("**Catppuccin {} Color Palette**", flavor_enum.to_string().to_uppercase());
                let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                let progress_msg = "📤 Uploading palette preview...";
                progress_bar.set_message(progress_msg);
                if let Err(e) = msg.channel_id.send_files(&ctx, vec![attachment_data], message_builder).await {
                    progress_bar.finish_with_message("❌ Failed to send palette preview");
                    error!(?e, "Failed to send palette preview for flavor: {}", flavor);
                    let _ = msg.channel_id.say(&ctx, "❌ Failed to send palette preview. Please try again later.").await;
                } else {
                    progress_bar.finish_with_message("✅ Palette preview uploaded successfully!");
                }
                return Ok(());
            }
        }
        progress_bar.finish_with_message("❌ Invalid palette command");
        let _ = msg.channel_id.say(&ctx, "❌ Invalid palette command. Use `!cat palette [flavor]` or `!cat palette all`. Try `!cat help` for more info.").await;
        return Ok(());
    }

    // List command
    if parts.get(0).map_or(false, |&p| p == "list") {
        // Start typing indicator for list command
        let _typing = msg.channel_id.start_typing(&ctx.http);
        
        // Create progress bar for list command
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {wide_msg}")
                .unwrap()
        );
        progress_bar.set_message("📋 Preparing available options list...");
        progress_bar.enable_steady_tick(Duration::from_millis(100));
        
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
        let progress_msg = "📤 Sending options list...";
        progress_bar.set_message(progress_msg);
        let _ = msg.channel_id.say(&ctx, message).await;
        progress_bar.finish_with_message("✅ Options list sent successfully!");
        return Ok(());
    }

    // Check for cancel subcommand
    if parts.get(0).map_or(false, |&p| p == "cancel") {
        // Start typing indicator for cancel command
        let _typing = msg.channel_id.start_typing(&ctx.http);
        
        // Create progress bar for cancel command
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {wide_msg}")
                .unwrap()
        );
        progress_bar.set_message("🛑 Cancelling your job...");
        progress_bar.enable_steady_tick(Duration::from_millis(100));
        
        let user_id = msg.author.id;
        let flag = CANCEL_FLAGS.entry(user_id).or_insert_with(|| Arc::new(std::sync::atomic::AtomicBool::new(false)));
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
        let progress_msg = "📤 Sending cancellation confirmation...";
        progress_bar.set_message(progress_msg);
        let _ = msg.channel_id.say(&ctx, "🛑 Your Catppuccinify job will be cancelled if running.").await;
        progress_bar.finish_with_message("✅ Cancellation request processed!");
        return Ok(());
    }

    // Random color or palette command
    if parts.get(0).map_or(false, |&p| p == "random") {
        // Start typing indicator for random commands
        let _typing = msg.channel_id.start_typing(&ctx.http);
        
        // Create progress bar for random commands
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {wide_msg}")
                .unwrap()
        );
        progress_bar.set_message("🎲 Generating random Catppuccin content...");
        progress_bar.enable_steady_tick(Duration::from_millis(100));
        
        use rand::seq::SliceRandom;
        use catppuccin::PALETTE;
        let flavors = ["latte", "frappe", "macchiato", "mocha"];
        if parts.get(1).map_or(false, |&p| p == "palette") {
            // Random palette preview
            let progress_msg = "🎨 Generating random palette preview...";
            progress_bar.set_message(progress_msg);
            let flavor = flavors.choose(&mut rand::thread_rng()).unwrap();
            let flavor_enum = utils::parse_flavor(flavor).unwrap();
            let palette_img = palette::generate_palette_preview(flavor_enum);
            let mut output_buffer = std::io::Cursor::new(Vec::new());
            if let Err(_e) = palette_img.write_to(&mut output_buffer, image::ImageFormat::Png) {
                progress_bar.finish_with_message("❌ Failed to generate palette preview");
                let _ = msg.channel_id.say(&ctx, "❌ Failed to generate palette preview.").await;
                return Ok(());
            }
            let filename = utils::sanitize_filename(&format!("catppuccin_palette_{}.png", flavor), "png");
            let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename);
            let message_content = format!("**Random Catppuccin Palette: {}**", flavor.to_uppercase());
            let message_builder = serenity::builder::CreateMessage::new().content(message_content);
            let progress_msg = "📤 Uploading random palette...";
            progress_bar.set_message(progress_msg);
            let _ = msg.channel_id.send_files(&ctx, vec![attachment_data], message_builder).await;
            progress_bar.finish_with_message("✅ Random palette uploaded successfully!");
            return Ok(());
    } else {
            // Random color
            let progress_msg = "🎨 Selecting random color...";
            progress_bar.set_message(progress_msg);
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
            let progress_msg = "📤 Sending random color...";
            progress_bar.set_message(progress_msg);
            let _ = msg.channel_id.say(&ctx, message).await;
            progress_bar.finish_with_message("✅ Random color sent successfully!");
            return Ok(());
        }
    }

    // Parse command arguments for flavor, algorithm, quality, format, etc.
    let mut selected_flavor = utils::parse_flavor("latte").unwrap();
    let mut selected_algorithm = "shepards-method"; // Default algorithm
    let mut batch_mode = false;
    let selected_format = None;

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
        // Start typing indicator for batch processing
        let _typing = msg.channel_id.start_typing(&ctx.http);
        
        // Create progress bar for batch processing
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {wide_msg}")
                .unwrap()
        );
        progress_bar.set_message("🔄 Starting batch processing...");
        progress_bar.enable_steady_tick(Duration::from_millis(100));
        
        let mut processed_attachments = Vec::new();
        let mut failed_count = 0;
        for (_i, attachment) in msg.attachments.iter().enumerate() {
            progress_bar.set_message("📥 Processing image...");
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
            let filename = utils::sanitize_filename(&format!("catppuccinified_{}_{}.", selected_flavor.to_string().to_lowercase(), attachment.filename), output_format.extensions_str().first().unwrap_or(&"png"));
            let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename);
            processed_attachments.push(attachment_data);
        }
        if !processed_attachments.is_empty() {
            let _processed_count = processed_attachments.len();
            progress_bar.set_message("📤 Uploading batch processed images...");
            let message_content = if failed_count > 0 {
                format!("Here are your Catppuccinified images! ({} failed)", failed_count)
            } else {
                "Here are your Catppuccinified images!".to_string()
            };
            let message_builder = serenity::builder::CreateMessage::new().content(message_content);
            let _ = msg.channel_id.send_files(&ctx, processed_attachments, message_builder).await;
            progress_bar.finish_with_message("✅ Batch processing completed!");
        } else {
            progress_bar.finish_with_message("❌ Failed to process any images. Please ensure your attachments are valid images.");
            let _ = msg.channel_id.say(&ctx, "Failed to process any images. Please ensure your attachments are valid images.").await;
        }
        return Ok(());
    }

    // For now, focus on image processing for a single image attachment or image URL
    // Start typing indicator and keep it active during processing
    let _typing = msg.channel_id.start_typing(&ctx.http);
    
    // Validate user input length
    if arg_string.len() > 300 {
        let _ = msg.channel_id.say(&ctx, "❌ Command too long. Please keep your command under 300 characters.").await;
        return Ok(());
    }
    // Check for image URL in arguments
    let url_regex = Regex::new(r"^(https?://[\w\-./%?=&]+\.(png|jpe?g|gif|bmp|webp))$").unwrap();
    let discord_msg_link_regex = Regex::new(r"^https://discord(?:app)?\.com/channels/(\d+)/(\d+)/(\d+)$").unwrap();
    let url_arg = parts.iter().find(|s| url_regex.is_match(s));
    let discord_link_arg = parts.iter().find(|s| discord_msg_link_regex.is_match(s));
    let mut image_url: Option<String> = None;
    let mut image_bytes: Option<bytes::Bytes> = None;
    let mut image_filename: Option<String> = None;
    if let Some(&url) = url_arg {
        if url.len() > 300 {
            let _ = msg.channel_id.say(&ctx, "❌ Image URL is too long.").await;
            return Ok(());
        }
        image_url = Some(url.to_string());
    }
    // If no direct image URL, check for Discord message link
    else if let Some(&discord_link) = discord_link_arg {
        if let Some(caps) = discord_msg_link_regex.captures(discord_link) {
            let channel_id = caps.get(2).unwrap().as_str().parse::<u64>().ok();
            let message_id = caps.get(3).unwrap().as_str().parse::<u64>().ok();
            if let (Some(channel_id), Some(message_id)) = (channel_id, message_id) {
                let channel_id = serenity::model::id::ChannelId(channel_id);
                let message_id = serenity::model::id::MessageId(message_id);
                match channel_id.message(&ctx.http, message_id).await {
                    Ok(fetched_msg) => {
                        // Try attachments first
                        if let Some(attachment) = fetched_msg.attachments.iter().find(|a| a.width.is_some() && a.height.is_some() && a.content_type.as_deref().map_or(false, |s| s.starts_with("image/"))) {
                            image_url = Some(attachment.url.clone());
                            image_filename = Some(attachment.filename.clone());
                        } else {
                            // Try embeds (image or thumbnail)
                            for embed in &fetched_msg.embeds {
                                if let Some(url) = embed.image.as_ref().and_then(|img| img.url.as_ref()) {
                                    image_url = Some(url.clone());
                                    break;
                                }
                                if let Some(url) = embed.thumbnail.as_ref().and_then(|img| img.url.as_ref()) {
                                    image_url = Some(url.clone());
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = msg.channel_id.say(&ctx, format!("❌ Failed to fetch message from link: {e}")).await;
                        return Ok(());
                    }
                }
            }
        }
    }
    let attachment = msg.attachments.iter().find(|a| a.width.is_some() && a.height.is_some());
    let image_source = if let Some(attachment) = attachment {
        Some((attachment.url.as_str().to_string(), Some(attachment.filename.clone())))
    } else if let Some(url) = image_url {
        Some((url, image_filename))
    } else {
        None
    };
    if let Some((image_url, filename)) = image_source {
        info!(url = %image_url, "Processing image from URL or attachment");
        
        // Create progress bar for console output
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {wide_msg}")
                .unwrap()
        );
        progress_bar.set_message("🔄 Starting image processing...");
        progress_bar.enable_steady_tick(Duration::from_millis(100));
        
        // Download the image
        progress_bar.set_message("📥 Downloading image...");
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
                progress_bar.set_message("✅ Image downloaded successfully");
                if image_bytes.len() > 8 * 1024 * 1024 {
                    progress_bar.finish_with_message("❌ Image is too large. Maximum allowed size is 8 MB.");
                    let _ = msg.channel_id.say(&ctx, "❌ Image is too large. Maximum allowed size is 8 MB.").await;
                    return Ok(());
                }
                progress_bar.set_message("🔍 Analyzing image format...");
                let img_reader = ImageReader::new(std::io::Cursor::new(&image_bytes)).with_guessed_format();
                if let Ok(reader) = img_reader {
                    let format = reader.format();
                    if let Some(image::ImageFormat::Gif) = format {
                        // Animated GIF: process all frames
                        progress_bar.set_message("🎬 Detected animated GIF - processing all frames...");
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
                                progress_bar.set_message("✅ GIF processing completed successfully");
                                let filename = utils::sanitize_filename(&format!("catppuccinified_{}.gif", selected_flavor.to_string().to_lowercase()), "gif");
                                let attachment_data = serenity::builder::CreateAttachment::bytes(gif_bytes, filename);
                                let message_content = format!("**Catppuccinified GIF with {}**", selected_flavor.to_string());
                                let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                                progress_bar.set_message("📤 Uploading processed GIF...");
                                if let Err(e) = msg.channel_id.send_files(&ctx, vec![attachment_data], message_builder).await {
                                    progress_bar.finish_with_message("❌ Failed to send processed GIF");
                                    error!(?e, "Failed to send processed GIF");
                                    let _ = msg.channel_id.say(&ctx, "❌ Failed to send processed GIF. Please try again later.").await;
                                } else {
                                    progress_bar.finish_with_message("✅ GIF uploaded successfully!");
                                }
                            }
                            Ok(Err(e)) => {
                                progress_bar.finish_with_message("❌ Failed to process GIF");
                                error!(?e, "Failed to process GIF");
                                let _ = msg.channel_id.say(&ctx, &format!("❌ Failed to process GIF: {e}")).await;
                            }
                                Err(e) => {
                                progress_bar.finish_with_message("❌ GIF processing panicked or failed to run");
                                error!(?e, "GIF processing panicked or failed to run");
                                let _ = msg.channel_id.say(&ctx, "❌ GIF processing failed unexpectedly. Please try again or contact the bot maintainer.").await;
                            }
                        }
                        return Ok(());
                    }
                    if let Ok(img) = reader.decode() {
                        progress_bar.set_message("✅ Image decoded successfully");
                        let (width, height) = img.dimensions();
                        progress_bar.set_message("📐 Image dimensions analyzed");
                        if width > 4096 || height > 4096 {
                            progress_bar.finish_with_message("❌ Image dimensions are too large. Maximum allowed is 4096x4096 pixels.");
                            let _ = msg.channel_id.say(&ctx, "❌ Image dimensions are too large. Maximum allowed is 4096x4096 pixels.").await;
                            return Ok(());
                        }
                        // Process the image using the selected flavor and algorithm
                        progress_bar.set_message("🎨 Processing with flavor and algorithm...");
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
                                progress_bar.set_message("✅ Image processing completed successfully");
                                let filename = utils::sanitize_filename(&format!("catppuccinified_{}.png", selected_flavor.to_string().to_lowercase()), "png");
                                let attachment_data = serenity::builder::CreateAttachment::bytes(image_bytes, filename);
                                let message_content = format!("**Catppuccinified with {}**", selected_flavor.to_string());
                                let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                                progress_bar.set_message("📤 Uploading processed image...");
                                if let Err(e) = msg.channel_id.send_files(&ctx, vec![attachment_data], message_builder).await {
                                    progress_bar.finish_with_message("❌ Failed to send processed image");
                                    error!(?e, "Failed to send processed image");
                                    let _ = msg.channel_id.say(&ctx, "❌ Failed to send processed image. Please try again later.").await;
                                } else {
                                    progress_bar.finish_with_message("✅ Image uploaded successfully!");
                                }
                            }
                            Ok(Err(e)) => {
                                if e.kind() == std::io::ErrorKind::Interrupted {
                                    progress_bar.finish_with_message("🛑 Your Catppuccinify job was cancelled.");
                                    let _ = msg.channel_id.say(&ctx, "🛑 Your Catppuccinify job was cancelled.").await;
                                } else {
                                    progress_bar.finish_with_message("❌ Failed to write processed image");
                                    error!(?e, "Failed to write processed image");
                                    let _ = msg.channel_id.say(&ctx, "❌ Failed to process image after conversion. Please try a different image or contact the bot maintainer.").await;
                                }
                            }
                            Err(e) => {
                                progress_bar.finish_with_message("❌ Image processing panicked or failed to run");
                                error!(?e, "Image processing panicked or failed to run");
                                let _ = msg.channel_id.say(&ctx, "❌ Image processing failed unexpectedly. Please try again or contact the bot maintainer.").await;
                            }
                        }
                        return Ok(());
                    }
                    progress_bar.finish_with_message("❌ Failed to decode image");
                    error!(url = %image_url, "Failed to decode image");
                    let _ = msg.channel_id.say(&ctx, "❌ Failed to decode the image. Please ensure your image is a supported format (PNG, JPEG, etc.) and not corrupted.").await;
                    return Ok(());
                } else {
                    progress_bar.finish_with_message("❌ Failed to create image reader");
                    error!(url = %image_url, "Failed to create image reader");
                    let _ = msg.channel_id.say(&ctx, "❌ Failed to read the image. Please try a different image or format.").await;
                    return Ok(());
                }
            } else {
                progress_bar.finish_with_message("❌ Failed to download image bytes");
                error!(url = %image_url, "Failed to download image bytes");
                let _ = msg.channel_id.say(&ctx, "❌ Failed to download the image. Please check the URL or try re-uploading your image.").await;
                return Ok(());
            }
        } else {
            progress_bar.finish_with_message("❌ Failed to fetch image from URL");
            error!(url = %image_url, "Failed to fetch image from URL");
            let _ = msg.channel_id.say(&ctx, "❌ Failed to fetch the image from the provided URL. Please check the URL and try again.").await;
            return Ok(());
        }
        return Ok(());
    }
    warn!(user = %msg.author.name, "No image attachment or valid URL found");
    let _ = msg.channel_id.say(&ctx, "❌ No image attachment or valid image URL found. Please attach an image, provide a direct image URL (ending in .png, .jpg, .jpeg, .gif, .bmp, .webp), or a Discord message link containing an image.").await;
    return Ok(());
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

    // Spawn a task to listen for shutdown signals
    let token_clone = token.clone();
    tokio::spawn(async move {
        // Wait for Ctrl+C or SIGTERM
        let _ = signal::ctrl_c().await;
        let http = serenity::http::Http::new(&token_clone);
        let channel_ids = [
            serenity::model::id::ChannelId::from(1393064541063221319u64),
            serenity::model::id::ChannelId::from(465193124852138011u64),
        ];
        for channel_id in channel_ids.iter() {
            let _ = channel_id.say(&http, "🔴 Catppuccinifier Bot is now offline!").await;
        }
        // Give the message a moment to send
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        std::process::exit(0);
    });

    if let Err(why) = client.start().await {
        info!(?why, "Client error");
    }
    Ok(())
}