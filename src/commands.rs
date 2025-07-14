// src/commands.rs

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use crate::utils;
use crate::palette;
use crate::image_processing;
use image::ImageReader;
use regex;
use tracing::{info, warn, error, debug};
use crate::utils::MOCHA_MAUVE;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub struct Handler;

// Helper function to send help message
pub async fn send_help_message(ctx: &Context, channel_id: serenity::model::id::ChannelId) -> Result<(), serenity::Error> {
    let help_parts = vec![
        r#"**Catppuccinifier Bot Commands**

**Basic Usage:**
`!cat [image]` - Process image with default Latte flavor
`!cat [flavor] [image]` - Process image with specific flavor
`!cat [flavor] [algorithm] [image]` - Process image with flavor and algorithm

**Hex Color Conversion:**
`!cat #FF0000` - Convert hex color to Catppuccin
`!cat [flavor] #FF0000` - Convert hex color with specific flavor

**Color Palette Preview:**
`!cat palette [flavor]` - Show all colors in a specific flavor
`!cat palette all` - Show all flavors' color palettes

**Before/After Comparison:**
`!cat compare [image]` - Send original + processed image side by side"#,
        r#"**Batch Processing:**
`!cat batch [multiple images]` - Process multiple images at once

**Quality Settings:**
`!cat [flavor] [quality] [image]` - quality: fast, normal, high

**Color Statistics:**
`!cat stats [image]` - Show dominant colors and suggest best flavor

**Export Options:**
`!cat [flavor] [format] [image]` - format: png, jpg, webp

**All Flavors Processing:**
`!cat all [image]` - Process image with all 4 flavors (Latte, Frappe, Macchiato, Mocha)"#,
        r#"**Available Flavors:**
• `latte` - Light, warm theme
• `frappe` - Medium, balanced theme  
• `macchiato` - Dark, rich theme
• `mocha` - Darkest, deep theme

**Available Algorithms:**
• `shepards` - Best quality (default)
• `gaussian` - Smooth gradients
• `linear` - Fast processing
• `sampling` - High quality, slower
• `nearest` - Fastest, basic
• `hald` - Hald CLUT method
• `euclide` - Euclidean distance
• `mean` - Mean-based mapping
• `std` - Standard deviation method"#,
        r#"**Quality Levels:**
• `fast` - Nearest neighbor (fastest)
• `normal` - Shepard's method (balanced)
• `high` - Gaussian sampling (best quality)

**Export Formats:**
• `png` - Lossless, supports transparency
• `jpg` - Compressed, smaller files
• `webp` - Modern, good compression
• `gif` - Animated images"#,
        r#"**Examples:**
`!cat mocha shepards [image]` - Mocha flavor with Shepard's method
`!cat frappe gaussian [image]` - Frappe flavor with Gaussian algorithm
`!cat all [image]` - Process with all flavors at once
`!cat palette latte` - Show Latte color palette
`!cat compare [image]` - Show before/after comparison
`!cat mocha high [image]` - High quality Mocha processing
`!cat latte png [image]` - Export as PNG format

**Help:**
`!cat -h` or `!cat help` - Show this help message"#
    ];
    for (i, help_part) in help_parts.iter().enumerate() {
        let part_number = if help_parts.len() > 1 {
            format!(" (Part {}/{})", i + 1, help_parts.len())
        } else {
            String::new()
        };
        let embed = serenity::builder::CreateEmbed::default()
            .description(format!("{}{}", help_part, part_number))
            .color(MOCHA_MAUVE);
        let builder = serenity::builder::CreateMessage::new().embed(embed);
        if let Err(why) = channel_id.send_message(&ctx.http, builder).await {
            error!(?why, "Error sending help message part {}", i + 1);
            break;
        }
        if i < help_parts.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }
    Ok(())
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        // Log every message event
        debug!(user = %msg.author.name, id = %msg.author.id, content = %msg.content, "Message event received");

        // Ignore messages from the bot itself or webhooks
        let current_user_id = ctx.http.get_current_user().await.unwrap().id;
        if msg.author.id == current_user_id {
            debug!(user = %msg.author.name, "Ignored message from self (bot user id)");
            return;
        }
        if msg.webhook_id.is_some() {
            debug!(user = %msg.author.name, "Ignored message from webhook");
            return;
        }
        if msg.author.bot {
            debug!(user = %msg.author.name, "Ignored message from bot user");
            return;
        }

        // Check if the message starts with our command prefix.
        if msg.content.starts_with("!cat") {
            info!(content = %msg.content, user = %msg.author.name, "Received !cat command");
            let parts: Vec<&str> = msg.content.split_whitespace().collect();

            // Handle help command
            if parts.len() > 1 && (parts[1] == "-h" || parts[1] == "--help" || parts[1] == "help") {
                // Start typing indicator for help
                let _typing = msg.channel_id.start_typing(&ctx.http);
                
                // Create progress bar for help
                let progress_bar = ProgressBar::new_spinner();
                progress_bar.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.green} {wide_msg}")
                        .unwrap()
                );
                progress_bar.set_message("📚 Preparing help message...");
                progress_bar.enable_steady_tick(Duration::from_millis(100));
                
                if let Err(why) = send_help_message(&ctx, msg.channel_id).await {
                    progress_bar.finish_with_message("❌ Error sending help message");
                    error!(?why, "Error sending help message");
                } else {
                    progress_bar.finish_with_message("✅ Help message sent successfully!");
                }
                return;
            }

            // Determine the flavor from the command arguments.
            let mut selected_flavor = utils::parse_flavor("latte").unwrap(); // Default flavor
            let mut has_explicit_flavor_arg = false;
            let mut selected_algorithm = "shepards-method"; // Default algorithm
            let mut process_all_flavors = false;
            let mut show_palette = false;
            let mut show_comparison = false;
            let mut show_stats = false;
            let mut batch_mode = false; // Now used for batch processing
            let mut selected_quality = None;
            let mut selected_format = None;

            if msg.content.split_whitespace().any(|arg| arg == "-f") {
                selected_quality = Some("fast".to_string());
                selected_algorithm = "nearest-neighbor";
                let _ = msg.channel_id.say(&ctx.http, "⚡ Fast mode enabled! Your image will be processed using the fastest settings (nearest-neighbor algorithm).").await;
            }

            if parts.len() > 1 {
                if parts[1] == "all" {
                    process_all_flavors = true;
                } else if parts[1] == "palette" {
                    show_palette = true;
                } else if parts[1] == "compare" {
                    show_comparison = true;
                } else if parts[1] == "stats" {
                    show_stats = true;
                } else if parts[1] == "batch" {
                    batch_mode = true;
                } else if let Some(flavor) = utils::parse_flavor(parts[1]) {
                    selected_flavor = flavor;
                    has_explicit_flavor_arg = true;
                } else if let Some(algorithm) = utils::parse_algorithm(parts[1]) {
                    selected_algorithm = algorithm;
                } else if let Some(quality) = utils::parse_quality(parts[1]) {
                    selected_quality = Some(quality.to_string());
                } else if let Some(format) = utils::parse_format(parts[1]) {
                    selected_format = Some(format);
                }
            }

            // Enable batch mode if multiple image attachments are present
            if msg.attachments.len() > 1 {
                batch_mode = true;
            }

            if parts.len() > 2 {
                if show_palette {
                    // Start typing indicator for palette generation
                    let _typing = msg.channel_id.start_typing(&ctx.http);
                    
                    // Create progress bar for palette generation
                    let progress_bar = ProgressBar::new_spinner();
                    progress_bar.set_style(
                        ProgressStyle::default_spinner()
                            .template("{spinner:.green} {wide_msg}")
                            .unwrap()
                    );
                    progress_bar.set_message("🎨 Generating palette preview...");
                    progress_bar.enable_steady_tick(Duration::from_millis(100));
                    
                    if parts[2] == "all" {
                        progress_bar.set_message("🎨 Generating all palette previews...");
                        let palette_img = palette::generate_all_palettes_preview();
                        let mut output_buffer = std::io::Cursor::new(Vec::new());
                        if let Err(_e) = palette_img.write_to(&mut output_buffer, image::ImageFormat::Png) {
                            progress_bar.finish_with_message("❌ Failed to generate palette preview");
                            let _ = msg.channel_id.say(&ctx.http, "Failed to generate palette preview.").await;
                            return;
                        }
                        let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), "catppuccin_palettes_all.png");
                        let message_content = "**All Catppuccin Color Palettes**\nFrom left to right: Latte, Frappe, Macchiato, Mocha";
                        let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                        progress_bar.set_message("📤 Uploading palette preview...");
                        let _ = msg.channel_id.send_files(&ctx.http, vec![attachment_data], message_builder).await;
                        progress_bar.finish_with_message("✅ All palette previews uploaded successfully!");
                        return;
                    } else if let Some(flavor) = utils::parse_flavor(parts[2]) {
                        progress_bar.set_message("🎨 Generating palette preview...");
                        let palette_img = palette::generate_palette_preview(flavor);
                        let mut output_buffer = std::io::Cursor::new(Vec::new());
                        if let Err(_e) = palette_img.write_to(&mut output_buffer, image::ImageFormat::Png) {
                            progress_bar.finish_with_message("❌ Failed to generate palette preview");
                            let _ = msg.channel_id.say(&ctx.http, "Failed to generate palette preview.").await;
                            return;
                        }
                        let filename = format!("catppuccin_palette_{}.png", flavor.to_string().to_lowercase());
                        let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename);
                        let message_content = format!("**Catppuccin {} Color Palette**", flavor.to_string().to_uppercase());
                        let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                        progress_bar.set_message("📤 Uploading palette preview...");
                        let _ = msg.channel_id.send_files(&ctx.http, vec![attachment_data], message_builder).await;
                        progress_bar.finish_with_message("✅ Palette preview uploaded successfully!");
                        return;
                    } else {
                        progress_bar.finish_with_message("❌ Invalid palette command");
                        let _ = msg.channel_id.say(&ctx.http, "Invalid palette command. Use `!cat palette [flavor]` or `!cat palette all`").await;
                        return;
                    }
                }
                if has_explicit_flavor_arg {
                    if let Some(algorithm) = utils::parse_algorithm(parts[2]) {
                        selected_algorithm = algorithm;
                    } else if let Some(quality) = utils::parse_quality(parts[2]) {
                        selected_quality = Some(quality.to_string());
                        selected_algorithm = quality;
                    } else if let Some(format) = utils::parse_format(parts[2]) {
                        selected_format = Some(format);
                    }
                }
            }

            // Hex Color Conversion Logic
            if msg.attachments.is_empty() {
                let input_color_arg_index = if has_explicit_flavor_arg { 2 } else { 1 };
                if parts.len() > input_color_arg_index {
                    let input_color = parts[input_color_arg_index];
                    let hex_regex = regex::Regex::new(r"^#?([0-9a-fA-F]{3}){1,2}$").unwrap();
                    if !hex_regex.is_match(input_color) {
                        let _ = msg.channel_id.say(&ctx.http, "That doesn't look like a valid hex color or flavor. Please use formats like `#FF0000` or `FF0000` for colors, or specify a flavor like `latte`, `frappe`, `macchiato`, `mocha` with an image.").await;
                        return;
                    }
                    // Start typing indicator for hex conversion
                    let _typing = msg.channel_id.start_typing(&ctx.http);
                    
                    // Create progress bar for hex conversion
                    let progress_bar = ProgressBar::new_spinner();
                    progress_bar.set_style(
                        ProgressStyle::default_spinner()
                            .template("{spinner:.green} {wide_msg}")
                            .unwrap()
                    );
                    progress_bar.set_message("🎨 Converting hex color to Catppuccin...");
                    progress_bar.enable_steady_tick(Duration::from_millis(100));
                    
                    match utils::find_closest_catppuccin_hex(input_color, selected_flavor) {
                        Some((color_name, converted_hex)) => {
                            progress_bar.set_message("✅ Color conversion completed");
                            let _embed_color = u32::from_str_radix(&converted_hex, 16).unwrap_or(0x000000);
                            let original_color_display = if input_color.starts_with('#') {
                                input_color.to_string()
                            } else {
                                format!("#{}", input_color)
                            };
                            let converted_color_display = format!("#{}", converted_hex);
                            let embed = serenity::builder::CreateEmbed::default()
                                .title("Catppuccin Color Conversion")
                                .description(format!("Original Color: `{}`", original_color_display))
                                .color(MOCHA_MAUVE)
                                .field(
                                    "Closest Catppuccin Color",
                                    format!("**{}** (`{}`) (Flavor: {})", color_name.to_uppercase(), converted_color_display, selected_flavor.to_string().to_uppercase()),
                                    false,
                                )
                                .field("\u{200b}", "**Color Swatch:** \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}", false);
                            let builder = serenity::builder::CreateMessage::new().embed(embed);
                            let _ = msg.channel_id.send_message(&ctx.http, builder).await;
                            progress_bar.finish_with_message("✅ Color conversion result sent!");
                        }
                        None => {
                            progress_bar.finish_with_message("❌ Error converting hex color");
                            let _ = msg.channel_id.say(&ctx.http, "Error converting hex color. Please ensure it's a valid 3 or 6 digit hex code.").await;
                        }
                    }
                    return;
                }
            }

            // Image Processing Logic
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
                
                // Batch processing: process all image attachments
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
                    progress_bar.set_message("📤 Uploading batch processed images...");
                    let message_content = if failed_count > 0 {
                        format!("Here are your Catppuccinified images! ({} failed)", failed_count)
                    } else {
                        "Here are your Catppuccinified images!".to_string()
                    };
                    let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                    let _processed_count = processed_attachments.len();
                    let _ = msg.channel_id.send_files(&ctx.http, processed_attachments, message_builder).await;
                    progress_bar.finish_with_message("✅ Batch processing completed!");
                } else {
                    progress_bar.finish_with_message("❌ Failed to process any images. Please ensure your attachments are valid images.");
                }
                return;
            }
            if let Some(attachment) = msg.attachments.first() {
                info!(filename = %attachment.filename, url = %attachment.url, "Image received");
                
                // Start typing indicator
                let _typing = msg.channel_id.start_typing(&ctx.http);
                
                // Create progress bar for console output
                let progress_bar = ProgressBar::new_spinner();
                progress_bar.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.green} {wide_msg}")
                        .unwrap()
                );
                progress_bar.set_message("🔄 Starting image processing...");
                progress_bar.enable_steady_tick(Duration::from_millis(100));
                
                // Only process if it's an image
                let content_type_is_image = attachment.content_type.as_deref().map_or(false, |s| s.starts_with("image/"));
                if !content_type_is_image {
                    progress_bar.finish_with_message("❌ Attachment is not an image");
                    warn!(?attachment.content_type, "Attachment is not an image");
                    let _ = msg.channel_id.say(&ctx.http, "Please attach an image to catppuccinify it.").await;
                    return;
                }

                // Download the image
                progress_bar.set_message("📥 Downloading image...");
                info!(url = %attachment.url, "Downloading image");
                let reqwest_client = reqwest::Client::new();
                let image_bytes = match reqwest_client.get(&attachment.url).send().await {
                    Ok(response) => match response.bytes().await {
                        Ok(bytes) => {
                            progress_bar.set_message("✅ Image downloaded successfully");
                            bytes
                        },
                        Err(_) => {
                            progress_bar.finish_with_message("❌ Failed to read image data");
                            error!("Failed to read image data");
                            let _ = msg.channel_id.say(&ctx.http, "Failed to read image data.").await;
                            return;
                        }
                    },
                    Err(_) => {
                        progress_bar.finish_with_message("❌ Failed to download image from Discord");
                        error!("Failed to download image from Discord");
                        let _ = msg.channel_id.say(&ctx.http, "Failed to download image from Discord.").await;
                        return;
                    }
                };

                // Load the image from bytes
                progress_bar.set_message("🔍 Decoding image...");
                info!("Decoding image");
                let img = match ImageReader::new(std::io::Cursor::new(image_bytes)).with_guessed_format().expect("Failed to guess image format").decode() {
                    Ok(img) => {
                        progress_bar.set_message("✅ Image decoded successfully");
                        img
                    },
                    Err(_) => {
                        progress_bar.finish_with_message("❌ Failed to decode the image");
                        error!("Failed to decode the image");
                        let _ = msg.channel_id.say(&ctx.http, "Failed to decode the image. Is it a valid image file?").await;
                        return;
                    }
                };

                // Convert to RGBA
                progress_bar.set_message("🔄 Converting image to RGBA...");
                debug!("Converting image to RGBA");
                let mut rgba_img = img.to_rgba8();
                let (width, height) = rgba_img.dimensions();
                progress_bar.set_message("📐 Image dimensions analyzed");

                // Handle color statistics
                if show_stats {
                    progress_bar.set_message("🎨 Analyzing image colors...");
                    info!("Analyzing image colors");
                    let (dominant_colors, suggested_flavor) = image_processing::analyze_image_colors(&rgba_img);
                    progress_bar.set_message("📊 Generating color statistics...");
                    let mut stats_message = format!("**Color Analysis Results**\n\n**Dominant Colors:**\n");
                    for (i, (r, g, b, count)) in dominant_colors.iter().enumerate() {
                        let hex = format!("{:02X}{:02X}{:02X}", r, g, b);
                        let percentage = (*count as f32 / (width * height) as f32 * 100.0).round() as u32;
                        stats_message.push_str(&format!("{}. `#{}` (RGB: {},{},{}) - {}%\n", i + 1, hex, r, g, b, percentage));
                    }
                    stats_message.push_str(&format!("\n**Suggested Flavor:** {}\n", suggested_flavor.to_string().to_uppercase()));
                    stats_message.push_str("\n*Based on average brightness of dominant colors*");
                    progress_bar.finish_with_message("✅ Color analysis completed");
                    let _ = msg.channel_id.say(&ctx.http, stats_message).await;
                    return;
                }

                if process_all_flavors {
                    progress_bar.set_message("🎨 Processing image with all flavors...");
                    info!("Processing image with all flavors");
                    let flavors = [
                        (utils::parse_flavor("latte").unwrap(), "latte"),
                        (utils::parse_flavor("frappe").unwrap(), "frappe"),
                        (utils::parse_flavor("macchiato").unwrap(), "macchiato"),
                        (utils::parse_flavor("mocha").unwrap(), "mocha")
                    ];
                    let mut attachments = Vec::new();
                    for (_i, (flavor, flavor_name)) in flavors.iter().enumerate() {
                        progress_bar.set_message("🎨 Processing with flavor...");
                        info!(flavor = %flavor_name, "Processing image with flavor");
                        let mut flavor_img = rgba_img.clone();
                        let lut = image_processing::generate_catppuccin_lut(*flavor, selected_algorithm);
                        image_processing::apply_lut_to_image(&mut flavor_img, &lut);
                        let mut output_buffer = std::io::Cursor::new(Vec::new());
                        let output_format = selected_format.unwrap_or(image::ImageFormat::Png);
                        let dynamic_img = image::DynamicImage::ImageRgba8(flavor_img);
                        if let Err(_e) = dynamic_img.write_to(&mut output_buffer, output_format) {
                            error!(flavor = %flavor_name, "Failed to encode processed image");
                            continue;
                        }
                        let filename = format!("catppuccinified_{}.{}", flavor_name, output_format.extensions_str().first().unwrap_or(&"png"));
                        let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename);
                        attachments.push(attachment_data);
                    }
                    if !attachments.is_empty() {
                        progress_bar.set_message("📤 Uploading all processed images...");
                        info!(count = attachments.len(), "Uploading all processed images");
                        let message_content = "Here are your Catppuccinified images with all flavors!";
                        let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                        let _ = msg.channel_id.send_files(&ctx.http, attachments, message_builder).await;
                        progress_bar.finish_with_message("✅ All flavors processed and uploaded successfully!");
                    } else {
                        progress_bar.finish_with_message("❌ Failed to process any flavors");
                    }
                    return;
                }

                // Single flavor processing
                progress_bar.set_message("🎨 Processing with flavor and algorithm...");
                info!(flavor = ?selected_flavor, "Processing image with selected flavor");
                let lut = image_processing::generate_catppuccin_lut(selected_flavor, selected_algorithm);
                image_processing::apply_lut_to_image(&mut rgba_img, &lut);

                // Handle comparison mode
                if show_comparison {
                    progress_bar.set_message("🔄 Creating before/after comparison image...");
                    info!("Creating before/after comparison image");
                    let original_img = img.to_rgba8();
                    let comparison_img = image_processing::create_comparison_image(&original_img, &rgba_img);
                    let mut output_buffer = std::io::Cursor::new(Vec::new());
                    let output_format = selected_format.unwrap_or(image::ImageFormat::Png);
                    if let Err(_e) = comparison_img.write_to(&mut output_buffer, output_format) {
                        progress_bar.finish_with_message("❌ Failed to create comparison image");
                        error!("Failed to create comparison image");
                        let _ = msg.channel_id.say(&ctx.http, "Failed to create comparison image.").await;
                        return;
                    }
                    let filename = format!("comparison_{}.{}", selected_flavor.to_string().to_lowercase(), output_format.extensions_str().first().unwrap_or(&"png"));
                    let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename);
                    let message_content = format!("**Before/After Comparison**\nLeft: Original | Right: {} flavor", selected_flavor.to_string().to_uppercase());
                    let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                    progress_bar.set_message("📤 Uploading comparison image...");
                    info!("Uploading comparison image");
                    let _ = msg.channel_id.send_files(&ctx.http, vec![attachment_data], message_builder).await;
                    progress_bar.finish_with_message("✅ Comparison image uploaded successfully!");
                    return;
                }

                // Save the processed image to a buffer
                progress_bar.set_message("💾 Encoding processed image...");
                let mut output_buffer = std::io::Cursor::new(Vec::new());
                let output_format = selected_format.unwrap_or(image::ImageFormat::Png);
                let dynamic_img = image::DynamicImage::ImageRgba8(rgba_img);
                if let Err(_e) = dynamic_img.write_to(&mut output_buffer, output_format) {
                    progress_bar.finish_with_message("❌ Failed to encode the processed image");
                    error!("Failed to encode the processed image");
                    let _ = msg.channel_id.say(&ctx.http, "Failed to encode the processed image.").await;
                    return;
                }
                let filename = format!("catppuccinified_{}.{}", selected_flavor.to_string().to_lowercase(), output_format.extensions_str().first().unwrap_or(&"png"));
                let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename.clone());
                let mut message_content = format!("Here's your Catppuccinified image (Flavor: {})!", selected_flavor.to_string().to_uppercase());
                if let Some(quality) = selected_quality {
                    message_content.push_str(&format!(" Quality: {}", quality));
                }
                if let Some(format) = selected_format {
                    message_content.push_str(&format!(" Format: {}", format.extensions_str().first().unwrap_or(&"unknown")));
                }
                let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                progress_bar.set_message("📤 Uploading processed image...");
                info!("Uploading processed image");
                let _ = msg.channel_id.send_files(&ctx.http, vec![attachment_data], message_builder).await;
                progress_bar.finish_with_message("✅ Image uploaded successfully!");
            }
        }
    }
    async fn ready(&self, ctx: Context, ready: serenity::model::gateway::Ready) {
        info!("{} is connected!", ready.user.name);
        info!("Bot is ready!");
        // Announce online in the specified channel
        let channel_id = serenity::model::id::ChannelId::from(1393064541063221319u64);
        let _ = channel_id.say(&ctx.http, "🟢 Catppuccinifier Bot is now online!").await;
    }
} 