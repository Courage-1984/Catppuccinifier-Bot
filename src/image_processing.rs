// src/image_processing.rs

use rayon::prelude::*;
use image::{RgbaImage, Rgba};
use catppuccin::{PALETTE, FlavorName};
use palette::{Lab, Srgb, IntoColor, color_difference::EuclideanDistance};

pub fn generate_catppuccin_lut(flavor: FlavorName, algorithm: &str) -> Vec<u8> {
    let colors_struct = match flavor {
        FlavorName::Latte => &PALETTE.latte.colors,
        FlavorName::Frappe => &PALETTE.frappe.colors,
        FlavorName::Macchiato => &PALETTE.macchiato.colors,
        FlavorName::Mocha => &PALETTE.mocha.colors,
    };
    let catppuccin_colors = [
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
    let catppuccin_labs: Vec<Lab> = catppuccin_colors.iter()
        .map(|color| {
            let (r, g, b) = (color.rgb.r, color.rgb.g, color.rgb.b);
            Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0).into_color()
        })
        .collect();
    let mut lut = vec![0u8; 256 * 256 * 256 * 3];
    let (_iterations, power, use_weighted) = match algorithm {
        "shepards-method" => (100, 2.0, true),
        "gaussian-rbf" => (50, 1.5, true),
        "linear-rbf" => (30, 1.0, false),
        "gaussian-sampling" => (200, 2.5, true),
        "nearest-neighbor" => (1, 1.0, false),
        "hald" => (150, 2.0, true),
        "euclide" => (80, 1.0, false),
        "mean" => (60, 1.5, true),
        "std" => (90, 2.0, true),
        _ => (100, 2.0, true),
    };
    for r_idx in 0..256 {
        for g_idx in 0..256 {
            for b_idx in 0..256 {
                let r = r_idx as f32 / 255.0;
                let g = g_idx as f32 / 255.0;
                let b = b_idx as f32 / 255.0;
                let input_lab: Lab = Srgb::new(r, g, b).into_color();
                let closest_color = if use_weighted {
                    let mut total_weight = 0.0;
                    let mut weighted_r = 0.0;
                    let mut weighted_g = 0.0;
                    let mut weighted_b = 0.0;
                    for (i, cat_lab) in catppuccin_labs.iter().enumerate() {
                        let distance = input_lab.distance_squared(*cat_lab);
                        let weight = if distance > 0.0 { 1.0 / distance.powf(power) } else { 1e6 };
                        let (cr, cg, cb) = (
                            catppuccin_colors[i].rgb.r as f32 / 255.0,
                            catppuccin_colors[i].rgb.g as f32 / 255.0,
                            catppuccin_colors[i].rgb.b as f32 / 255.0,
                        );
                        weighted_r += cr * weight;
                        weighted_g += cg * weight;
                        weighted_b += cb * weight;
                        total_weight += weight;
                    }
                    if total_weight > 0.0 {
                        (
                            (weighted_r / total_weight * 255.0).clamp(0.0, 255.0) as u8,
                            (weighted_g / total_weight * 255.0).clamp(0.0, 255.0) as u8,
                            (weighted_b / total_weight * 255.0).clamp(0.0, 255.0) as u8,
                        )
                    } else {
                        (catppuccin_colors[0].rgb.r, catppuccin_colors[0].rgb.g, catppuccin_colors[0].rgb.b)
                    }
                } else {
                    let mut min_distance = f32::MAX;
                    let mut closest_color = catppuccin_colors[0];
                    for (i, cat_lab) in catppuccin_labs.iter().enumerate() {
                        let distance = input_lab.distance_squared(*cat_lab);
                        if distance < min_distance {
                            min_distance = distance;
                            closest_color = catppuccin_colors[i];
                        }
                    }
                    (closest_color.rgb.r, closest_color.rgb.g, closest_color.rgb.b)
                };
                let lut_idx = (r_idx * 256 * 256 + g_idx * 256 + b_idx) * 3;
                lut[lut_idx] = closest_color.0;
                lut[lut_idx + 1] = closest_color.1;
                lut[lut_idx + 2] = closest_color.2;
            }
        }
    }
    lut
}

pub fn sample_lut(lut: &[u8], r: f32, g: f32, b: f32) -> [f32; 3] {
    let r_idx = ((r * 255.0).clamp(0.0, 255.0) as usize).min(255);
    let g_idx = ((g * 255.0).clamp(0.0, 255.0) as usize).min(255);
    let b_idx = ((b * 255.0).clamp(0.0, 255.0) as usize).min(255);
    let idx = (r_idx * 256 * 256 + g_idx * 256 + b_idx) * 3;
    if idx + 2 < lut.len() {
        [
            lut[idx] as f32 / 255.0,
            lut[idx + 1] as f32 / 255.0,
            lut[idx + 2] as f32 / 255.0,
        ]
    } else {
        [r, g, b]
    }
}

pub fn apply_lut_to_image(img: &mut RgbaImage, lut: &[u8]) {
    let (width, _height) = img.dimensions();
    let pixels: Vec<(u32, u32, Rgba<u8>)> = img.pixels()
        .enumerate()
        .map(|(i, pixel)| {
            let x = i as u32 % width;
            let y = i as u32 / width;
            (x, y, *pixel)
        })
        .collect();
    let transformed_pixels: Vec<(u32, u32, Rgba<u8>)> = pixels
        .par_iter()
        .map(|(x, y, pixel)| {
            let r = pixel[0] as f32 / 255.0;
            let g = pixel[1] as f32 / 255.0;
            let b = pixel[2] as f32 / 255.0;
            let a = pixel[3];
            let transformed = sample_lut(lut, r, g, b);
            let new_r = (transformed[0] * 255.0).clamp(0.0, 255.0) as u8;
            let new_g = (transformed[1] * 255.0).clamp(0.0, 255.0) as u8;
            let new_b = (transformed[2] * 255.0).clamp(0.0, 255.0) as u8;
            (*x, *y, Rgba([new_r, new_g, new_b, a]))
        })
        .collect();
    for (x, y, pixel) in transformed_pixels {
        img.put_pixel(x, y, pixel);
    }
}

pub fn create_comparison_image(original: &RgbaImage, processed: &RgbaImage) -> RgbaImage {
    let (orig_w, orig_h) = original.dimensions();
    let (proc_w, proc_h) = processed.dimensions();
    let max_width = orig_w.max(proc_w);
    let max_height = orig_h.max(proc_h);
    let margin = 20;
    let total_width = max_width * 2 + margin;
    let total_height = max_height;
    let mut comparison = RgbaImage::new(total_width, total_height);
    for x in 0..total_width {
        for y in 0..total_height {
            comparison.put_pixel(x, y, Rgba([240, 240, 240, 255]));
        }
    }
    for x in 0..orig_w {
        for y in 0..orig_h {
            comparison.put_pixel(x, y, *original.get_pixel(x, y));
        }
    }
    for x in 0..proc_w {
        for y in 0..proc_h {
            comparison.put_pixel(max_width + margin + x, y, *processed.get_pixel(x, y));
        }
    }
    comparison
}

pub fn analyze_image_colors(img: &RgbaImage) -> (Vec<(u8, u8, u8, u32)>, FlavorName) {
    let mut color_counts = std::collections::HashMap::new();
    for pixel in img.pixels() {
        let key = (pixel[0], pixel[1], pixel[2]);
        *color_counts.entry(key).or_insert(0) += 1;
    }
    let mut sorted_colors: Vec<_> = color_counts.into_iter().collect();
    sorted_colors.sort_by(|a, b| b.1.cmp(&a.1));
    let dominant_colors: Vec<(u8, u8, u8, u32)> = sorted_colors
        .into_iter()
        .take(5)
        .map(|((r, g, b), count)| (r, g, b, count))
        .collect();
    let avg_brightness: f32 = dominant_colors.iter()
        .map(|(r, g, b, _)| (*r as f32 + *g as f32 + *b as f32) / 3.0)
        .sum::<f32>() / dominant_colors.len() as f32;
    let suggested_flavor = if avg_brightness > 180.0 {
        FlavorName::Latte
    } else if avg_brightness > 120.0 {
        FlavorName::Frappe
    } else if avg_brightness > 80.0 {
        FlavorName::Macchiato
    } else {
        FlavorName::Mocha
    };
    (dominant_colors, suggested_flavor)
} 