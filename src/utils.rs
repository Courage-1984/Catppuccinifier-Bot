// src/utils.rs

use serenity::model::channel::Message;
use serenity::prelude::*;
use image::ImageFormat;
use catppuccin::FlavorName;

// Parse a string into a Catppuccin FlavorName enum
pub fn parse_flavor(s: &str) -> Option<FlavorName> {
    match s.to_lowercase().as_str() {
        "latte" => Some(FlavorName::Latte),
        "frappe" => Some(FlavorName::Frappe),
        "macchiato" => Some(FlavorName::Macchiato),
        "mocha" => Some(FlavorName::Mocha),
        _ => None,
    }
}

// Parse algorithm from string
pub fn parse_algorithm(s: &str) -> Option<&'static str> {
    match s.to_lowercase().as_str() {
        "shepards" | "shepards-method" | "shepard" => Some("shepards-method"),
        "gaussian" | "gaussian-rbf" | "rbf" => Some("gaussian-rbf"),
        "linear" | "linear-rbf" => Some("linear-rbf"),
        "sampling" | "gaussian-sampling" | "gauss" => Some("gaussian-sampling"),
        "nearest" | "nearest-neighbor" | "nn" => Some("nearest-neighbor"),
        "hald" => Some("hald"),
        "euclide" => Some("euclide"),
        "mean" => Some("mean"),
        "std" => Some("std"),
        _ => None,
    }
}

// Parse quality setting
pub fn parse_quality(s: &str) -> Option<&'static str> {
    match s.to_lowercase().as_str() {
        "fast" => Some("nearest-neighbor"),
        "normal" => Some("shepards-method"),
        "high" => Some("gaussian-sampling"),
        _ => None,
    }
}

// Parse export format
pub fn parse_format(s: &str) -> Option<ImageFormat> {
    match s.to_lowercase().as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "webp" => Some(ImageFormat::WebP),
        "gif" => Some(ImageFormat::Gif),
        _ => None,
    }
}

// Find closest Catppuccin color for a given hex string
pub fn find_closest_catppuccin_hex(input_hex: &str, _flavor: FlavorName) -> Option<(String, String)> {
    let hex_str = input_hex.trim_start_matches('#');
    let (_r, _g, _b) = if hex_str.len() == 6 {
        (
            u8::from_str_radix(&hex_str[0..2], 16).ok()?,
            u8::from_str_radix(&hex_str[2..4], 16).ok()?,
            u8::from_str_radix(&hex_str[4..6], 16).ok()?,
        )
    } else if hex_str.len() == 3 {
        (
            u8::from_str_radix(&hex_str[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&hex_str[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&hex_str[2..3].repeat(2), 16).ok()?,
        )
    } else {
        return None;
    };
    // Use LUT for hex color conversion too
    // This will be updated to call image_processing::generate_catppuccin_lut after migration
    None // Placeholder, update after moving LUT logic
}

#[allow(dead_code)]
pub async fn update_progress_message(
    ctx: &Context,
    channel_id: serenity::model::id::ChannelId,
    message: &mut Message,
    progress_text: &str,
) -> Result<(), serenity::Error> {
    let embed = serenity::builder::CreateEmbed::default()
        .title("🔄 Catppuccinifier Bot - Processing")
        .description(progress_text)
        .color(0x89b4fa)
        .footer(serenity::builder::CreateEmbedFooter::new("Processing your image..."));
    let builder = serenity::builder::EditMessage::new().embed(embed);
    match message.edit(&ctx.http, builder).await {
        Ok(_) => Ok(()),
        Err(_) => {
            let new_embed = serenity::builder::CreateEmbed::default()
                .title("🔄 Catppuccinifier Bot - Processing")
                .description(progress_text)
                .color(0x89b4fa)
                .footer(serenity::builder::CreateEmbedFooter::new("Processing your image..."));
            let new_builder = serenity::builder::CreateMessage::new().embed(new_embed);
            channel_id.send_message(&ctx.http, new_builder).await?;
            Ok(())
        }
    }
}

#[allow(dead_code)]
pub async fn send_success_message(
    ctx: &Context,
    _channel_id: serenity::model::id::ChannelId,
    message: &mut Message,
    success_text: &str,
) -> Result<(), serenity::Error> {
    let embed = serenity::builder::CreateEmbed::default()
        .title("✅ Catppuccinifier Bot - Complete")
        .description(success_text)
        .color(0xa6e3a1)
        .footer(serenity::builder::CreateEmbedFooter::new("Processing complete!"));
    let builder = serenity::builder::EditMessage::new().embed(embed);
    message.edit(&ctx.http, builder).await?;
    Ok(())
} 