// src/palette.rs

use image::RgbaImage;
use catppuccin::{PALETTE, FlavorName};
use image::Rgba;

pub fn generate_palette_preview(flavor: FlavorName) -> RgbaImage {
    let colors_struct = match flavor {
        FlavorName::Latte => &PALETTE.latte.colors,
        FlavorName::Frappe => &PALETTE.frappe.colors,
        FlavorName::Macchiato => &PALETTE.macchiato.colors,
        FlavorName::Mocha => &PALETTE.mocha.colors,
    };
    let colors = [
        colors_struct.rosewater, colors_struct.flamingo, colors_struct.pink,
        colors_struct.mauve, colors_struct.red, colors_struct.maroon,
        colors_struct.peach, colors_struct.yellow, colors_struct.green,
        colors_struct.teal, colors_struct.sky, colors_struct.sapphire,
        colors_struct.blue, colors_struct.lavender, colors_struct.text,
        colors_struct.subtext1, colors_struct.subtext0, colors_struct.overlay2,
        colors_struct.overlay1, colors_struct.overlay0, colors_struct.surface2,
        colors_struct.surface1, colors_struct.surface0, colors_struct.base,
        colors_struct.mantle, colors_struct.crust,
    ];
    let swatch_size: u32 = 60;
    let grid_size: u32 = 5;
    let margin: u32 = 10;
    let total_size = grid_size * swatch_size + (grid_size + 1) * margin;
    let mut img = RgbaImage::new(total_size, total_size);
    for (i, color) in colors.iter().enumerate() {
        if i >= 25 { break; }
        let row = (i as u32) / grid_size;
        let col = (i as u32) % grid_size;
        let x = margin + col * (swatch_size + margin);
        let y = margin + row * (swatch_size + margin);
        for px in x..x + swatch_size {
            for py in y..y + swatch_size {
                img.put_pixel(px, py, Rgba([color.rgb.r, color.rgb.g, color.rgb.b, 255]));
            }
        }
    }
    img
}

pub fn generate_all_palettes_preview() -> RgbaImage {
    let flavors = [FlavorName::Latte, FlavorName::Frappe, FlavorName::Macchiato, FlavorName::Mocha];
    let swatch_size: u32 = 40;
    let margin: u32 = 5;
    let colors_per_flavor: u32 = 26;
    let grid_cols: u32 = 5;
    let grid_rows: u32 = 6; // 5x6=30, enough for 26 colors
    let flavor_width = grid_cols * swatch_size + (grid_cols + 1) * margin;
    let flavor_height = grid_rows * swatch_size + (grid_rows + 1) * margin + 30;
    let total_width = flavor_width * 4 + margin * 5;
    let total_height = flavor_height;
    let mut img = RgbaImage::new(total_width, total_height);
    for (flavor_idx, flavor) in flavors.iter().enumerate() {
        let colors_struct = match flavor {
            FlavorName::Latte => &PALETTE.latte.colors,
            FlavorName::Frappe => &PALETTE.frappe.colors,
            FlavorName::Macchiato => &PALETTE.macchiato.colors,
            FlavorName::Mocha => &PALETTE.mocha.colors,
        };
        let colors = [
            colors_struct.rosewater, colors_struct.flamingo, colors_struct.pink,
            colors_struct.mauve, colors_struct.red, colors_struct.maroon,
            colors_struct.peach, colors_struct.yellow, colors_struct.green,
            colors_struct.teal, colors_struct.sky, colors_struct.sapphire,
            colors_struct.blue, colors_struct.lavender, colors_struct.text,
            colors_struct.subtext1, colors_struct.subtext0, colors_struct.overlay2,
            colors_struct.overlay1, colors_struct.overlay0, colors_struct.surface2,
            colors_struct.surface1, colors_struct.surface0, colors_struct.base,
            colors_struct.mantle, colors_struct.crust,
        ];
        let flavor_x = margin + (flavor_idx as u32) * (flavor_width + margin);
        for i in 0..flavor_width {
            for j in 0..30 {
                img.put_pixel(flavor_x + i, j, Rgba([255, 255, 255, 255]));
            }
        }
        for (i, color) in colors.iter().enumerate() {
            let row = (i as u32) / grid_cols;
            let col = (i as u32) % grid_cols;
            let x = flavor_x + margin + col * (swatch_size + margin);
            let y = 30 + margin + row * (swatch_size + margin);
            for px in x..x + swatch_size {
                for py in y..y + swatch_size {
                    img.put_pixel(px, py, Rgba([color.rgb.r, color.rgb.g, color.rgb.b, 255]));
                }
            }
        }
    }
    img
}

/// Generate a horizontal gradient image from a list of RGB tuples
pub fn generate_gradient_image(colors: &[(u8, u8, u8)], width: u32, height: u32) -> image::RgbaImage {
    let n = colors.len();
    let mut img = image::RgbaImage::new(width, height);
    if n == 0 {
        return img;
    }
    for x in 0..width {
        // Determine which segment this x falls into
        let t = x as f32 / (width - 1) as f32;
        let seg = if n == 1 {
            0
        } else {
            ((t * (n as f32 - 1.0)).floor() as usize).min(n - 2)
        };
        let local_t = if n == 1 {
            0.0
        } else {
            (t * (n as f32 - 1.0)) - seg as f32
        };
        let (r1, g1, b1) = colors[seg];
        let (r2, g2, b2) = if seg + 1 < n { colors[seg + 1] } else { colors[seg] };
        let r = (r1 as f32 * (1.0 - local_t) + r2 as f32 * local_t).round() as u8;
        let g = (g1 as f32 * (1.0 - local_t) + g2 as f32 * local_t).round() as u8;
        let b = (b1 as f32 * (1.0 - local_t) + b2 as f32 * local_t).round() as u8;
        for y in 0..height {
            img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
        }
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;
    use catppuccin::FlavorName;

    #[test]
    fn test_generate_palette_preview_dimensions() {
        let img = generate_palette_preview(FlavorName::Latte);
        // 5x5 grid, swatch_size 60, margin 10: total = 5*60 + 6*10 = 360
        assert_eq!(img.width(), 360);
        assert_eq!(img.height(), 360);
    }

    #[test]
    fn test_generate_palette_preview_pixel_color() {
        let img = generate_palette_preview(FlavorName::Latte);
        // Top-left swatch should be rosewater
        let px = img.get_pixel(10 + 30, 10 + 30); // center of first swatch
        let colors_struct = &catppuccin::PALETTE.latte.colors;
        let rosewater = [colors_struct.rosewater.rgb.r, colors_struct.rosewater.rgb.g, colors_struct.rosewater.rgb.b, 255];
        assert_eq!(px.0, rosewater);
    }

    #[test]
    fn test_generate_all_palettes_preview_dimensions() {
        let img = generate_all_palettes_preview();
        // 4 flavors, each flavor_width = 4*40 + 5*5 = 185, total_width = 4*185 + 5*5 = 765
        // flavor_height = 16*40 + 17*5 + 30 = 755
        assert_eq!(img.width(), 765);
        assert_eq!(img.height(), 755);
    }
} 