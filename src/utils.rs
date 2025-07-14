// src/utils.rs

use serenity::model::channel::Message;
use serenity::prelude::*;
use image::ImageFormat;
use catppuccin::FlavorName;

// Catppuccin Mocha theme colors
pub const MOCHA_MAUVE: u32 = 0xcba6f7; // accent
pub const MOCHA_GREEN: u32 = 0xa6e3a1; // success
pub const MOCHA_BLUE: u32 = 0x89b4fa; // info/progress
pub const MOCHA_RED: u32 = 0xf38ba8; // error

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
pub fn find_closest_catppuccin_hex(input_hex: &str, flavor: FlavorName) -> Option<(String, String)> {
    let hex_str = input_hex.trim_start_matches('#');
    let (r, g, b) = if hex_str.len() == 6 {
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
    let colors_struct = match flavor {
        FlavorName::Latte => &catppuccin::PALETTE.latte.colors,
        FlavorName::Frappe => &catppuccin::PALETTE.frappe.colors,
        FlavorName::Macchiato => &catppuccin::PALETTE.macchiato.colors,
        FlavorName::Mocha => &catppuccin::PALETTE.mocha.colors,
    };
    let palette = [
        ("rosewater", colors_struct.rosewater), ("flamingo", colors_struct.flamingo), ("pink", colors_struct.pink),
        ("mauve", colors_struct.mauve), ("red", colors_struct.red), ("maroon", colors_struct.maroon),
        ("peach", colors_struct.peach), ("yellow", colors_struct.yellow), ("green", colors_struct.green),
        ("teal", colors_struct.teal), ("sky", colors_struct.sky), ("sapphire", colors_struct.sapphire),
        ("blue", colors_struct.blue), ("lavender", colors_struct.lavender), ("text", colors_struct.text),
        ("subtext1", colors_struct.subtext1), ("subtext0", colors_struct.subtext0), ("overlay2", colors_struct.overlay2),
        ("overlay1", colors_struct.overlay1), ("overlay0", colors_struct.overlay0), ("surface2", colors_struct.surface2),
        ("surface1", colors_struct.surface1), ("surface0", colors_struct.surface0), ("base", colors_struct.base),
        ("mantle", colors_struct.mantle), ("crust", colors_struct.crust),
    ];
    let mut min_dist = f32::MAX;
    let mut closest = &palette[0];
    for (name, color) in &palette {
        let dr = *r as f32 - color.rgb.r as f32;
        let dg = *g as f32 - color.rgb.g as f32;
        let db = *b as f32 - color.rgb.b as f32;
        let dist = dr * dr + dg * dg + db * db;
        if dist < min_dist {
            min_dist = dist;
            closest = &(*name, *color);
        }
    }
    let hex = format!("{:02X}{:02X}{:02X}", closest.1.rgb.r, closest.1.rgb.g, closest.1.rgb.b);
    Some((closest.0.to_string(), hex))
}

// Parse a Catppuccin color name to its RGB tuple for a given flavor
pub fn catppuccin_color_name_to_rgb(name: &str, flavor: FlavorName) -> Option<(u8, u8, u8)> {
    let colors_struct = match flavor {
        FlavorName::Latte => &catppuccin::PALETTE.latte.colors,
        FlavorName::Frappe => &catppuccin::PALETTE.frappe.colors,
        FlavorName::Macchiato => &catppuccin::PALETTE.macchiato.colors,
        FlavorName::Mocha => &catppuccin::PALETTE.mocha.colors,
    };
    match name.to_lowercase().as_str() {
        "rosewater" => Some((colors_struct.rosewater.rgb.r, colors_struct.rosewater.rgb.g, colors_struct.rosewater.rgb.b)),
        "flamingo" => Some((colors_struct.flamingo.rgb.r, colors_struct.flamingo.rgb.g, colors_struct.flamingo.rgb.b)),
        "pink" => Some((colors_struct.pink.rgb.r, colors_struct.pink.rgb.g, colors_struct.pink.rgb.b)),
        "mauve" => Some((colors_struct.mauve.rgb.r, colors_struct.mauve.rgb.g, colors_struct.mauve.rgb.b)),
        "red" => Some((colors_struct.red.rgb.r, colors_struct.red.rgb.g, colors_struct.red.rgb.b)),
        "maroon" => Some((colors_struct.maroon.rgb.r, colors_struct.maroon.rgb.g, colors_struct.maroon.rgb.b)),
        "peach" => Some((colors_struct.peach.rgb.r, colors_struct.peach.rgb.g, colors_struct.peach.rgb.b)),
        "yellow" => Some((colors_struct.yellow.rgb.r, colors_struct.yellow.rgb.g, colors_struct.yellow.rgb.b)),
        "green" => Some((colors_struct.green.rgb.r, colors_struct.green.rgb.g, colors_struct.green.rgb.b)),
        "teal" => Some((colors_struct.teal.rgb.r, colors_struct.teal.rgb.g, colors_struct.teal.rgb.b)),
        "sky" => Some((colors_struct.sky.rgb.r, colors_struct.sky.rgb.g, colors_struct.sky.rgb.b)),
        "sapphire" => Some((colors_struct.sapphire.rgb.r, colors_struct.sapphire.rgb.g, colors_struct.sapphire.rgb.b)),
        "blue" => Some((colors_struct.blue.rgb.r, colors_struct.blue.rgb.g, colors_struct.blue.rgb.b)),
        "lavender" => Some((colors_struct.lavender.rgb.r, colors_struct.lavender.rgb.g, colors_struct.lavender.rgb.b)),
        "text" => Some((colors_struct.text.rgb.r, colors_struct.text.rgb.g, colors_struct.text.rgb.b)),
        "subtext1" => Some((colors_struct.subtext1.rgb.r, colors_struct.subtext1.rgb.g, colors_struct.subtext1.rgb.b)),
        "subtext0" => Some((colors_struct.subtext0.rgb.r, colors_struct.subtext0.rgb.g, colors_struct.subtext0.rgb.b)),
        "overlay2" => Some((colors_struct.overlay2.rgb.r, colors_struct.overlay2.rgb.g, colors_struct.overlay2.rgb.b)),
        "overlay1" => Some((colors_struct.overlay1.rgb.r, colors_struct.overlay1.rgb.g, colors_struct.overlay1.rgb.b)),
        "overlay0" => Some((colors_struct.overlay0.rgb.r, colors_struct.overlay0.rgb.g, colors_struct.overlay0.rgb.b)),
        "surface2" => Some((colors_struct.surface2.rgb.r, colors_struct.surface2.rgb.g, colors_struct.surface2.rgb.b)),
        "surface1" => Some((colors_struct.surface1.rgb.r, colors_struct.surface1.rgb.g, colors_struct.surface1.rgb.b)),
        "surface0" => Some((colors_struct.surface0.rgb.r, colors_struct.surface0.rgb.g, colors_struct.surface0.rgb.b)),
        "base" => Some((colors_struct.base.rgb.r, colors_struct.base.rgb.g, colors_struct.base.rgb.b)),
        "mantle" => Some((colors_struct.mantle.rgb.r, colors_struct.mantle.rgb.g, colors_struct.mantle.rgb.b)),
        "crust" => Some((colors_struct.crust.rgb.r, colors_struct.crust.rgb.g, colors_struct.crust.rgb.b)),
        _ => None,
    }
}

// Sanitize a filename for safe output (removes dangerous characters, enforces extension, limits length)
pub fn sanitize_filename(filename: &str, default_ext: &str) -> String {
    use regex::Regex;
    // Remove any path separators and non-printable characters
    let re = Regex::new(r#"[^A-Za-z0-9._-]"#).unwrap();
    let mut name = re.replace_all(filename, "_").to_string();
    // Remove leading/trailing dots/underscores/hyphens
    name = name.trim_matches(|c: char| c == '.' || c == '_' || c == '-').to_string();
    // Limit length
    if name.len() > 64 {
        name.truncate(64);
    }
    // Ensure extension
    if !name.contains('.') {
        name.push('.');
        name.push_str(default_ext);
    } else if let Some(ext) = name.split('.').last() {
        if ext.len() > 8 || ext.is_empty() {
            name.push('.');
            name.push_str(default_ext);
        }
    }
    name
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
        .color(MOCHA_BLUE)
        .footer(serenity::builder::CreateEmbedFooter::new("Processing your image..."));
    let builder = serenity::builder::EditMessage::new().embed(embed);
    match message.edit(&ctx.http, builder).await {
        Ok(_) => Ok(()),
        Err(_) => {
            let new_embed = serenity::builder::CreateEmbed::default()
                .title("🔄 Catppuccinifier Bot - Processing")
                .description(progress_text)
                .color(MOCHA_BLUE)
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
        .color(MOCHA_GREEN)
        .footer(serenity::builder::CreateEmbedFooter::new("Processing complete!"));
    let builder = serenity::builder::EditMessage::new().embed(embed);
    message.edit(&ctx.http, builder).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_flavor() {
        assert_eq!(parse_flavor("latte").unwrap().to_string(), "Latte");
        assert_eq!(parse_flavor("frappe").unwrap().to_string(), "Frappé");
        assert_eq!(parse_flavor("macchiato").unwrap().to_string(), "Macchiato");
        assert_eq!(parse_flavor("mocha").unwrap().to_string(), "Mocha");
        assert!(parse_flavor("unknown").is_none());
    }

    #[test]
    fn test_parse_algorithm() {
        assert_eq!(parse_algorithm("shepards-method").unwrap(), "shepards-method");
        assert_eq!(parse_algorithm("nearest-neighbor").unwrap(), "nearest-neighbor");
        assert!(parse_algorithm("not-an-algo").is_none());
    }

    #[test]
    fn test_parse_format() {
        assert_eq!(parse_format("png").unwrap().extensions_str()[0], "png");
        assert_eq!(parse_format("jpg").unwrap().extensions_str()[0], "jpg");
        assert!(parse_format("not-a-format").is_none());
    }

    // Add more tests for color conversion helpers if present
} 