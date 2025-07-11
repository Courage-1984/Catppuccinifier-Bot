// src/image_processing.rs

use rayon::prelude::*;
use image::{RgbaImage, Rgba};
use catppuccin::{PALETTE, FlavorName};
use palette::{Lab, Srgb, IntoColor, color_difference::EuclideanDistance};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use gif::{Decoder as GifDecoder, Encoder as GifEncoder, Frame as GifFrame, Repeat};
use std::io::Cursor;

static LUT_CACHE: Lazy<Mutex<HashMap<(String, String), Arc<Vec<u8>>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn generate_catppuccin_lut(_flavor: FlavorName, _algorithm: &str) -> Arc<Vec<u8>> {
    let key = (_flavor.to_string(), _algorithm.to_string());
    {
        let cache = LUT_CACHE.lock().unwrap();
        if let Some(lut) = cache.get(&key) {
            return lut.clone();
        }
    }
    let colors_struct = match _flavor {
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
    let (_iterations, power, use_weighted) = match _algorithm {
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
    let lut_arc = Arc::new(lut);
    let mut cache = LUT_CACHE.lock().unwrap();
    cache.insert(key, lut_arc.clone());
    lut_arc
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

pub fn process_image_with_palette(img: &image::DynamicImage, _flavor: catppuccin::FlavorName, _algorithm: &str) -> image::DynamicImage {
    let lut = generate_catppuccin_lut(_flavor, _algorithm);
    let mut img_rgba = img.to_rgba8();
    apply_lut_to_image(&mut img_rgba, &lut);
    image::DynamicImage::ImageRgba8(img_rgba)
}

pub fn process_gif_with_palette(gif_bytes: &[u8], flavor: catppuccin::FlavorName, algorithm: &str) -> Result<Vec<u8>, String> {
    let mut decoder = GifDecoder::new(Cursor::new(gif_bytes)).map_err(|e| format!("Failed to create GIF decoder: {e}"))?;
    let global_palette = decoder.global_palette().map(|p| p.to_vec());
    let mut processed_frames = Vec::new();
    while let Some(frame) = decoder.read_next_frame().map_err(|e| format!("Failed to read GIF frame: {e}"))? {
        let width = frame.width as u16;
        let height = frame.height as u16;
        let palette = frame.palette.as_ref().map(|v| v.as_slice()).or(global_palette.as_ref().map(|v| v.as_slice()));
        println!("GIF frame: width={}, height={}, buffer_len={}, palette_len={}",
            width, height, frame.buffer.len(), palette.map(|p| p.len()).unwrap_or(0));
        // Convert indexed frame to RGBA
        let mut rgba_buf = Vec::with_capacity((width as usize) * (height as usize) * 4);
        if let Some(pal) = palette {
            for &idx in frame.buffer.iter() {
                let i = idx as usize * 3;
                if i + 2 < pal.len() {
                    rgba_buf.push(pal[i]);     // R
                    rgba_buf.push(pal[i + 1]); // G
                    rgba_buf.push(pal[i + 2]); // B
                    rgba_buf.push(255);        // A
                } else {
                    rgba_buf.extend_from_slice(&[0, 0, 0, 255]);
                }
            }
        } else {
            // No palette, treat as grayscale
            for &v in frame.buffer.iter() {
                rgba_buf.extend_from_slice(&[v, v, v, 255]);
            }
        }
        let mut rgba_img = image::RgbaImage::from_raw(width as u32, height as u32, rgba_buf)
            .ok_or("Failed to convert GIF frame to RGBA image")?;
        let lut = generate_catppuccin_lut(flavor, algorithm);
        apply_lut_to_image(&mut rgba_img, &lut);
        let mut processed_frame = GifFrame::from_rgba_speed(width, height, &mut rgba_img.into_raw(), 10);
        processed_frame.delay = frame.delay;
        processed_frames.push(processed_frame);
    }
    // Encode new GIF
    let mut output = Vec::new();
    if let Some(first_frame) = processed_frames.first() {
        let mut encoder = GifEncoder::new(&mut output, first_frame.width, first_frame.height, &[])
            .map_err(|e| format!("Failed to create GIF encoder: {e}"))?;
        encoder.set_repeat(Repeat::Infinite).map_err(|e| format!("Failed to set GIF repeat: {e}"))?;
        for frame in processed_frames {
            encoder.write_frame(&frame).map_err(|e| format!("Failed to write GIF frame: {e}"))?;
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use catppuccin::FlavorName;

    #[test]
    fn test_generate_catppuccin_lut_length() {
        let lut = generate_catppuccin_lut(FlavorName::Latte, "shepards-method");
        assert_eq!(lut.len(), 256 * 256 * 256 * 3);
    }

    #[test]
    fn test_generate_catppuccin_lut_different_flavors() {
        let lut1 = generate_catppuccin_lut(FlavorName::Latte, "shepards-method");
        let lut2 = generate_catppuccin_lut(FlavorName::Mocha, "shepards-method");
        assert_ne!(lut1[..100], lut2[..100]); // The LUTs should differ for different flavors
    }

    #[test]
    fn test_create_comparison_image() {
        use image::{RgbaImage, Rgba};
        let mut orig = RgbaImage::new(10, 10);
        let mut proc = RgbaImage::new(10, 10);
        for x in 0..10 {
            for y in 0..10 {
                orig.put_pixel(x, y, Rgba([255, 0, 0, 255]));
                proc.put_pixel(x, y, Rgba([0, 255, 0, 255]));
            }
        }
        let cmp = create_comparison_image(&orig, &proc);
        assert_eq!(cmp.width(), 10 * 2 + 20);
        assert_eq!(cmp.height(), 10);
        // Check left and right halves
        assert_eq!(cmp.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
        assert_eq!(cmp.get_pixel(10 + 20, 0), &Rgba([0, 255, 0, 255]));
    }

    #[test]
    fn test_process_gif_with_palette_minimal() {
        // Minimal 2-frame GIF (1x1 px, red and green)
        let gif_bytes: &[u8] = b"GIF89a\x01\x00\x01\x00\x80\x00\x00\xFF\x00\x00\x00\xFF\x00!\xF9\x04\x00\x00\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00!\xF9\x04\x00\x00\x00\x00\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00;";
        let result = process_gif_with_palette(gif_bytes, FlavorName::Latte, "shepards-method");
        if let Err(e) = &result {
            println!("GIF processing error: {}", e);
        }
        assert!(result.is_ok());
        let out = result.unwrap();
        assert!(!out.is_empty());
    }
} 