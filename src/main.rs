// src/main.rs

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::env;
use regex::Regex;
use image::{ImageReader, DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;
use reqwest::Client as ReqwestClient;
use dotenv::dotenv;
use std::time::Instant;
use rayon::prelude::*;
use palette::{Lab, Srgb, IntoColor, color_difference::EuclideanDistance};
use tokio::sync::{mpsc, oneshot, Semaphore};
use std::sync::Arc;
use tokio::sync::Mutex;

// Import the correct items from the 'catppuccin' crate
use catppuccin::{PALETTE, FlavorName}; // Changed Flavor to FlavorName

// Task types for the queue
#[derive(Debug)]
enum TaskType {
    ImageProcessing {
        flavor: FlavorName,
        algorithm: String,
        process_all_flavors: bool,
        show_comparison: bool,
        show_stats: bool,
        batch_mode: bool,
        selected_quality: Option<String>,
        selected_format: Option<ImageFormat>,
        attachment_url: String,
        content_type: Option<String>,
    },
    HexConversion {
        hex_color: String,
        flavor: FlavorName,
    },
    PalettePreview {
        flavor: Option<FlavorName>,
        show_all: bool,
    },
    Help,
}

// Task structure for the queue
#[derive(Debug)]
struct QueuedTask {
    id: u64,
    task_type: TaskType,
    user_id: u64,
    channel_id: u64,
    message_id: u64,
    response_tx: oneshot::Sender<()>,
}

// Queue manager structure
struct TaskQueue {
    tx: mpsc::Sender<QueuedTask>,
    next_id: Arc<Mutex<u64>>,
    queue_length: Arc<Mutex<usize>>,
}

impl TaskQueue {
    fn new() -> Self {
        let (tx, mut rx) = mpsc::channel::<QueuedTask>(100);
        let next_id = Arc::new(Mutex::new(1));
        let queue_length = Arc::new(Mutex::new(0));
        
        // Spawn the queue processor
        let next_id_clone = next_id.clone();
        let queue_length_clone = queue_length.clone();
        tokio::spawn(async move {
            let mut queue: Vec<QueuedTask> = Vec::new();
            let mut processing = false;
            
            loop {
                tokio::select! {
                    // Handle new task
                    task = rx.recv() => {
                        if let Some(task) = task {
                            let task_id = {
                                let mut id_guard = next_id_clone.lock().await;
                                let id = *id_guard;
                                *id_guard += 1;
                                id
                            };
                            
                            let mut task = task;
                            task.id = task_id;
                            queue.push(task);
                            
                            // Update queue length
                            {
                                let mut len_guard = queue_length_clone.lock().await;
                                *len_guard = queue.len();
                            }
                            
                            println!("Task {} queued. Queue length: {}", task_id, queue.len());
                        } else {
                            // Channel closed, break the loop
                            break;
                        }
                    }
                    
                    // Process next task if not currently processing
                    _ = async {
                        if !processing && !queue.is_empty() {
                            processing = true;
                        }
                    }, if !processing && !queue.is_empty() => {
                        if queue.is_empty() {
                            processing = false;
                        } else {
                            let task = queue.remove(0);
                            processing = true;
                            
                            // Update queue length
                            {
                                let mut len_guard = queue_length_clone.lock().await;
                                *len_guard = queue.len();
                            }
                            
                            // Process the task in a separate task to avoid blocking
                            tokio::spawn(async move {
                                process_task(task).await;
                            });
                        }
                    }
                    
                    // Mark processing as complete after a short delay
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)), if processing => {
                        processing = false;
                    }
                }
            }
        });
        
        Self { tx, next_id, queue_length }
    }
    
    async fn enqueue(&self, task: TaskType, user_id: u64, channel_id: u64, message_id: u64) -> Result<u64, mpsc::error::SendError<QueuedTask>> {
        let (response_tx, _response_rx) = oneshot::channel();
        let task_id = {
            let mut id_guard = self.next_id.lock().await;
            let id = *id_guard;
            *id_guard += 1;
            id
        };
        
        let queued_task = QueuedTask {
            id: task_id,
            task_type: task,
            user_id,
            channel_id,
            message_id,
            response_tx,
        };
        
        self.tx.send(queued_task).await?;
        Ok(task_id)
    }
    
    async fn get_queue_length(&self) -> usize {
        let len_guard = self.queue_length.lock().await;
        *len_guard
    }
}



// Global task queue and semaphore for controlling concurrent processing
static TASK_QUEUE: once_cell::sync::Lazy<Arc<TaskQueue>> = once_cell::sync::Lazy::new(|| {
    Arc::new(TaskQueue::new())
});

// Semaphore to limit concurrent processing
static PROCESSING_SEMAPHORE: once_cell::sync::Lazy<Arc<Semaphore>> = once_cell::sync::Lazy::new(|| {
    Arc::new(Semaphore::new(1)) // Only process one task at a time
});

// Process a single task
async fn process_task(task: QueuedTask) {
    println!("Processing task {} for user {}", task.id, task.user_id);
    
    match task.task_type {
        TaskType::ImageProcessing { flavor, algorithm: _, process_all_flavors: _, show_comparison: _, show_stats: _, batch_mode: _, selected_quality: _, selected_format: _, attachment_url: _, content_type: _ } => {
            println!("Processing image task {} with flavor {:?}", task.id, flavor);
            
            // Simulate processing time
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            println!("Completed image task {}", task.id);
        }
        TaskType::HexConversion { hex_color, flavor } => {
            println!("Processing hex conversion task {} for color {} with flavor {:?}", task.id, hex_color, flavor);
            
            // Simulate processing time
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            println!("Completed hex conversion task {}", task.id);
        }
        TaskType::PalettePreview { flavor, show_all } => {
            println!("Processing palette preview task {} for flavor {:?}, show_all: {}", task.id, flavor, show_all);
            
            // Simulate processing time
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            println!("Completed palette preview task {}", task.id);
        }
        TaskType::Help => {
            println!("Processing help task {}", task.id);
            
            // Simulate processing time
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            println!("Completed help task {}", task.id);
        }
    }
    
    // Send completion signal
    let _ = task.response_tx.send(());
}

// Helper function to send help message
async fn send_help_message(ctx: &Context, channel_id: serenity::model::id::ChannelId) -> Result<(), serenity::Error> {
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
        
        if let Err(why) = channel_id.say(&ctx.http, &format!("{}{}", help_part, part_number)).await {
            eprintln!("Error sending help message part {}: {:?}", i + 1, why);
            break;
        }
        
        // Small delay between messages to avoid rate limiting
        if i < help_parts.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    }
    
    Ok(())
}

// Helper function to send queue status message
async fn send_queue_status(
    ctx: &Context,
    channel_id: serenity::model::id::ChannelId,
    task_id: u64,
    position: usize,
) -> Result<(), serenity::Error> {
    let embed = serenity::builder::CreateEmbed::default()
        .title("⏳ Catppuccinifier Bot - Queued")
        .description(format!(
            "Your request has been queued!\n\n**Task ID:** {}\n**Position in queue:** {}\n\nI'll process your request as soon as possible. You'll be notified when it's your turn!",
            task_id, position
        ))
        .color(0xf5c2e7) // Catppuccin pink color
        .footer(serenity::builder::CreateEmbedFooter::new("Processing queue..."));

    let builder = serenity::builder::CreateMessage::new().embed(embed);
    channel_id.send_message(&ctx.http, builder).await?;
    Ok(())
}

struct Handler;

/// Helper function to parse a string into a Catppuccin FlavorName enum.
fn parse_flavor(s: &str) -> Option<FlavorName> {
    match s.to_lowercase().as_str() {
        "latte" => Some(FlavorName::Latte),
        "frappe" => Some(FlavorName::Frappe),
        "macchiato" => Some(FlavorName::Macchiato),
        "mocha" => Some(FlavorName::Mocha),
        _ => None,
    }
}

/// Helper function to parse algorithm from string
fn parse_algorithm(s: &str) -> Option<&'static str> {
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

/// Helper function to parse quality setting
fn parse_quality(s: &str) -> Option<&'static str> {
    match s.to_lowercase().as_str() {
        "fast" => Some("nearest-neighbor"),
        "normal" => Some("shepards-method"),
        "high" => Some("gaussian-sampling"),
        _ => None,
    }
}

/// Helper function to parse export format
fn parse_format(s: &str) -> Option<ImageFormat> {
    match s.to_lowercase().as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "webp" => Some(ImageFormat::WebP),
        "gif" => Some(ImageFormat::Gif),
        _ => None,
    }
}

/// Generate a color palette preview image
fn generate_palette_preview(flavor: FlavorName) -> RgbaImage {
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

    // Create a 5x5 grid of color swatches (25 colors total)
    let swatch_size: u32 = 60;
    let grid_size: u32 = 5;
    let margin: u32 = 10;
    let total_size = grid_size * swatch_size + (grid_size + 1) * margin;
    
    let mut img = RgbaImage::new(total_size, total_size);
    
    for (i, color) in colors.iter().enumerate() {
        if i >= 25 { break; } // Only show first 25 colors
        
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

/// Generate a combined palette preview for all flavors
fn generate_all_palettes_preview() -> RgbaImage {
    let flavors = [FlavorName::Latte, FlavorName::Frappe, FlavorName::Macchiato, FlavorName::Mocha];
    let _flavor_names = ["Latte", "Frappe", "Macchiato", "Mocha"];
    
    let swatch_size: u32 = 40;
    let margin: u32 = 5;
    let colors_per_flavor: u32 = 16; // Show first 16 colors per flavor
    let grid_cols: u32 = 4;
    let grid_rows: u32 = colors_per_flavor;
    
    let flavor_width = grid_cols * swatch_size + (grid_cols + 1) * margin;
    let flavor_height = grid_rows * swatch_size + (grid_rows + 1) * margin + 30; // Extra space for label
    
    let total_width = flavor_width * 4 + margin * 5; // 4 flavors + margins
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
            colors_struct.subtext1,
        ];
        
        let flavor_x = margin + (flavor_idx as u32) * (flavor_width + margin);
        
        // Add flavor name label (simple text rendering)
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

/// Create a side-by-side comparison image
fn create_comparison_image(original: &RgbaImage, processed: &RgbaImage) -> RgbaImage {
    let (orig_w, orig_h) = original.dimensions();
    let (proc_w, proc_h) = processed.dimensions();
    
    let max_width = orig_w.max(proc_w);
    let max_height = orig_h.max(proc_h);
    let margin = 20;
    
    let total_width = max_width * 2 + margin;
    let total_height = max_height;
    
    let mut comparison = RgbaImage::new(total_width, total_height);
    
    // Fill background
    for x in 0..total_width {
        for y in 0..total_height {
            comparison.put_pixel(x, y, Rgba([240, 240, 240, 255]));
        }
    }
    
    // Copy original image
    for x in 0..orig_w {
        for y in 0..orig_h {
            comparison.put_pixel(x, y, *original.get_pixel(x, y));
        }
    }
    
    // Copy processed image
    for x in 0..proc_w {
        for y in 0..proc_h {
            comparison.put_pixel(max_width + margin + x, y, *processed.get_pixel(x, y));
        }
    }
    
    comparison
}

/// Analyze image colors and suggest best flavor
fn analyze_image_colors(img: &RgbaImage) -> (Vec<(u8, u8, u8, u32)>, FlavorName) {
    let mut color_counts = std::collections::HashMap::new();
    
    // Count colors (simplified - just use RGB values)
    for pixel in img.pixels() {
        let key = (pixel[0], pixel[1], pixel[2]);
        *color_counts.entry(key).or_insert(0) += 1;
    }
    
    // Get top 5 dominant colors
    let mut sorted_colors: Vec<_> = color_counts.into_iter().collect();
    sorted_colors.sort_by(|a, b| b.1.cmp(&a.1));
    
    let dominant_colors: Vec<(u8, u8, u8, u32)> = sorted_colors
        .into_iter()
        .take(5)
        .map(|((r, g, b), count)| (r, g, b, count))
        .collect();
    
    // Simple heuristic to suggest flavor based on average brightness
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

/// Generate a LUT for the specified Catppuccin flavor using advanced color matching
fn generate_catppuccin_lut(flavor: FlavorName, algorithm: &str) -> Vec<u8> {
    let colors_struct = match flavor {
        FlavorName::Latte => &PALETTE.latte.colors,
        FlavorName::Frappe => &PALETTE.frappe.colors,
        FlavorName::Macchiato => &PALETTE.macchiato.colors,
        FlavorName::Mocha => &PALETTE.mocha.colors,
    };

    // Get all Catppuccin colors for this flavor
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

    // Convert Catppuccin colors to CIELAB for better perceptual matching
    let catppuccin_labs: Vec<Lab> = catppuccin_colors.iter()
        .map(|color| {
            let (r, g, b) = (color.rgb.r, color.rgb.g, color.rgb.b);
            Srgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0).into_color()
        })
        .collect();

    // Create a 256x256x256 LUT for higher precision
    let mut lut = vec![0u8; 256 * 256 * 256 * 3];
    
    // Algorithm-specific parameters
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

    // Generate LUT by sampling the entire RGB space with algorithm-specific logic
    for r_idx in 0..256 {
        for g_idx in 0..256 {
            for b_idx in 0..256 {
                let r = r_idx as f32 / 255.0;
                let g = g_idx as f32 / 255.0;
                let b = b_idx as f32 / 255.0;
                
                // Convert input RGB to CIELAB
                let input_lab: Lab = Srgb::new(r, g, b).into_color();
                
                let closest_color = if use_weighted {
                    // Weighted average approach (like Shepard's method)
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
                    // Nearest neighbor approach
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
                
                // Store the closest Catppuccin color in the LUT
                let lut_idx = (r_idx * 256 * 256 + g_idx * 256 + b_idx) * 3;
                lut[lut_idx] = closest_color.0;
                lut[lut_idx + 1] = closest_color.1;
                lut[lut_idx + 2] = closest_color.2;
            }
        }
    }
    
    lut
}

/// Sample from a LUT (using lutgen's sampling)
fn sample_lut(lut: &[u8], r: f32, g: f32, b: f32) -> [f32; 3] {
    // lutgen generates a 256x256x256 LUT, so we need to sample from it
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
        [r, g, b] // Fallback to original color
    }
}

/// Apply LUT-based color transformation to an image
fn apply_lut_to_image(img: &mut image::RgbaImage, lut: &[u8]) {
    let (width, _height) = img.dimensions();
    
    // Process pixels in parallel
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

            // Apply LUT transformation
            let transformed = sample_lut(lut, r, g, b);
            
            let new_r = (transformed[0] * 255.0).clamp(0.0, 255.0) as u8;
            let new_g = (transformed[1] * 255.0).clamp(0.0, 255.0) as u8;
            let new_b = (transformed[2] * 255.0).clamp(0.0, 255.0) as u8;

            (*x, *y, Rgba([new_r, new_g, new_b, a]))
        })
        .collect();

    // Write back transformed pixels
    for (x, y, pixel) in transformed_pixels {
        img.put_pixel(x, y, pixel);
    }
}

/// Helper function to find the closest Catppuccin color for a given hex string (for hex color conversion)
fn find_closest_catppuccin_hex(input_hex: &str, flavor: FlavorName) -> Option<(String, String)> {
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

    // Use LUT for hex color conversion too
    let lut = generate_catppuccin_lut(flavor, "shepards-method");
    let transformed = sample_lut(&lut, r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    
    let cr = (transformed[0] * 255.0).clamp(0.0, 255.0) as u8;
    let cg = (transformed[1] * 255.0).clamp(0.0, 255.0) as u8;
    let cb = (transformed[2] * 255.0).clamp(0.0, 255.0) as u8;
    
    let closest_hex = format!("{:02X}{:02X}{:02X}", cr, cg, cb);

    // Find the name of the closest color
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
    
    // Find closest color by distance
    let mut min_distance = f32::MAX;
    let mut closest_name = String::new();
    
    for color_entry in catppuccin_colors.iter() {
        let (cr_actual, cg_actual, cb_actual) = (color_entry.rgb.r, color_entry.rgb.g, color_entry.rgb.b);
        let distance = ((cr as f32 - cr_actual as f32).powi(2) + 
                       (cg as f32 - cg_actual as f32).powi(2) + 
                       (cb as f32 - cb_actual as f32).powi(2)).sqrt();
        
        if distance < min_distance {
            min_distance = distance;
            closest_name = color_entry.name.to_string();
        }
    }
    
    Some((closest_name, closest_hex))
}

/// Helper function to update progress message
async fn update_progress_message(
    ctx: &Context,
    channel_id: serenity::model::id::ChannelId,
    message: &mut serenity::model::channel::Message,
    progress_text: &str,
) -> Result<(), serenity::Error> {
    let embed = serenity::builder::CreateEmbed::default()
        .title("🔄 Catppuccinifier Bot - Processing")
        .description(progress_text)
        .color(0x89b4fa) // Catppuccin blue color
        .footer(serenity::builder::CreateEmbedFooter::new("Processing your image..."));

    let builder = serenity::builder::EditMessage::new().embed(embed);
    
    // Try to edit the original message, fall back to sending new message if editing fails
    match message.edit(&ctx.http, builder).await {
        Ok(_) => Ok(()),
        Err(_) => {
            // If editing fails, send a new message
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

/// Helper function to send final success message
async fn send_success_message(
    ctx: &Context,
    _channel_id: serenity::model::id::ChannelId,
    message: &mut serenity::model::channel::Message,
    success_text: &str,
) -> Result<(), serenity::Error> {
    let embed = serenity::builder::CreateEmbed::default()
        .title("✅ Catppuccinifier Bot - Complete")
        .description(success_text)
        .color(0xa6e3a1) // Catppuccin green color
        .footer(serenity::builder::CreateEmbedFooter::new("Processing complete!"));

    let builder = serenity::builder::EditMessage::new().embed(embed);
    message.edit(&ctx.http, builder).await?;
    Ok(())
}


#[async_trait]
impl EventHandler for Handler {
    // This event fires when a message is received.
    async fn message(&self, ctx: Context, msg: Message) {
        println!("Received message: {}", msg.content);
        // Ignore messages from the bot itself to prevent infinite loops.
        if msg.author.bot {
            println!("Ignored message from bot user: {}", msg.author.name);
            return;
        }

        // Check if the message starts with our command prefix.
        if msg.content.starts_with("!cat") {
            println!("Detected !cat command");
            let parts: Vec<&str> = msg.content.split_whitespace().collect();

            // Handle help command
            if parts.len() > 1 && (parts[1] == "-h" || parts[1] == "--help" || parts[1] == "help") {
                // Send help message immediately
                if let Err(why) = send_help_message(&ctx, msg.channel_id).await {
                    eprintln!("Error sending help message: {:?}", why);
                }
                return;
            }

            // Determine the flavor from the command arguments.
            let mut selected_flavor = FlavorName::Latte; // Default flavor
            let mut has_explicit_flavor_arg = false;
            let mut selected_algorithm = "shepards-method"; // Default algorithm
            let mut process_all_flavors = false;
            let mut show_palette = false;
            let mut show_comparison = false;
            let mut show_stats = false;
            let mut batch_mode = false;
            let mut selected_quality = None;
            let mut selected_format = None;

            if parts.len() > 1 {
                if parts[1] == "all" {
                    process_all_flavors = true;
                    println!("Processing with all flavors");
                } else if parts[1] == "palette" {
                    show_palette = true;
                    println!("Showing palette preview");
                } else if parts[1] == "compare" {
                    show_comparison = true;
                    println!("Showing comparison");
                } else if parts[1] == "stats" {
                    show_stats = true;
                    println!("Showing color statistics");
                } else if parts[1] == "batch" {
                    batch_mode = true;
                    println!("Batch processing mode");
                } else if let Some(flavor) = parse_flavor(parts[1]) {
                    selected_flavor = flavor;
                    has_explicit_flavor_arg = true;
                    println!("Selected flavor: {:?}", selected_flavor);
                } else if let Some(algorithm) = parse_algorithm(parts[1]) {
                    selected_algorithm = algorithm;
                    println!("Selected algorithm: {}", selected_algorithm);
                } else if let Some(quality) = parse_quality(parts[1]) {
                    selected_quality = Some(quality.to_string()); // Store as String
                    println!("Selected quality: {}", quality);
                } else if let Some(format) = parse_format(parts[1]) {
                    selected_format = Some(format);
                    println!("Selected format: {:?}", format);
                }
            }

            // Check for additional arguments
            if parts.len() > 2 {
                if show_palette {
                    // Acquire semaphore permit for processing
                    let _permit = PROCESSING_SEMAPHORE.acquire().await.unwrap();
                    
                    // Send queue status if there are other tasks waiting
                    let available_permits = PROCESSING_SEMAPHORE.available_permits();
                    if available_permits == 0 {
                        if let Err(why) = msg.channel_id.say(&ctx.http, "⏳ Your palette preview request is queued. I'll process it as soon as possible!").await {
                            eprintln!("Error sending queue message: {:?}", why);
                        }
                    }
                    
                    // Handle palette command: !cat palette [flavor]
                    if parts[2] == "all" {
                        // Show all palettes
                        let palette_img = generate_all_palettes_preview();
                        let mut output_buffer = Cursor::new(Vec::new());
                        
                        if let Err(e) = palette_img.write_to(&mut output_buffer, ImageFormat::Png) {
                            eprintln!("Error encoding palette image: {:?}", e);
                            if let Err(why) = msg.channel_id.say(&ctx.http, "Failed to generate palette preview.").await {
                                eprintln!("Error sending message: {:?}", why);
                            }
                            return;
                        }

                        let attachment_data = serenity::builder::CreateAttachment::bytes(
                            output_buffer.into_inner(), 
                            "catppuccin_palettes_all.png"
                        );

                        let message_content = "**All Catppuccin Color Palettes**\nFrom left to right: Latte, Frappe, Macchiato, Mocha";
                        let message_builder = serenity::builder::CreateMessage::new().content(message_content);

                        if let Err(why) = msg.channel_id.send_files(&ctx.http, vec![attachment_data], message_builder).await {
                            eprintln!("Error sending palette image: {:?}", why);
                        }
                        return;
                    } else if let Some(flavor) = parse_flavor(parts[2]) {
                        // Show specific flavor palette
                        let palette_img = generate_palette_preview(flavor);
                        let mut output_buffer = Cursor::new(Vec::new());
                        
                        if let Err(e) = palette_img.write_to(&mut output_buffer, ImageFormat::Png) {
                            eprintln!("Error encoding palette image: {:?}", e);
                            if let Err(why) = msg.channel_id.say(&ctx.http, "Failed to generate palette preview.").await {
                                eprintln!("Error sending message: {:?}", why);
                            }
                            return;
                        }

                        let filename = format!("catppuccin_palette_{}.png", flavor.to_string().to_lowercase());
                        let attachment_data = serenity::builder::CreateAttachment::bytes(
                            output_buffer.into_inner(), 
                            filename
                        );

                        let message_content = format!("**Catppuccin {} Color Palette**", flavor.to_string().to_uppercase());
                        let message_builder = serenity::builder::CreateMessage::new().content(message_content);

                        if let Err(why) = msg.channel_id.send_files(&ctx.http, vec![attachment_data], message_builder).await {
                            eprintln!("Error sending palette image: {:?}", why);
                        }
                        return;
                    } else {
                        if let Err(why) = msg.channel_id.say(&ctx.http, "Invalid palette command. Use `!cat palette [flavor]` or `!cat palette all`").await {
                            eprintln!("Error sending message: {:?}", why);
                        }
                        return;
                    }
                }
                
                if has_explicit_flavor_arg {
                    if let Some(algorithm) = parse_algorithm(parts[2]) {
                        selected_algorithm = algorithm;
                        println!("Selected algorithm: {}", selected_algorithm);
                    } else if let Some(quality) = parse_quality(parts[2]) {
                        selected_quality = Some(quality.to_string()); // Store as String
                        selected_algorithm = quality; // Use quality as algorithm
                        println!("Selected quality: {}", quality);
                    } else if let Some(format) = parse_format(parts[2]) {
                        selected_format = Some(format);
                        println!("Selected format: {:?}", format);
                    }
                }
            }

            // --- Hex Color Conversion Logic ---
            if msg.attachments.is_empty() {
                let input_color_arg_index = if has_explicit_flavor_arg { 2 } else { 1 };
                if parts.len() > input_color_arg_index {
                    let input_color = parts[input_color_arg_index];
                    println!("Hex color argument detected: {}", input_color);
                    
                    // Acquire semaphore permit for processing
                    let _permit = PROCESSING_SEMAPHORE.acquire().await.unwrap();
                    
                    // Send queue status if there are other tasks waiting
                    let available_permits = PROCESSING_SEMAPHORE.available_permits();
                    if available_permits == 0 {
                        if let Err(why) = msg.channel_id.say(&ctx.http, "⏳ Your hex color conversion is queued. I'll process it as soon as possible!").await {
                            eprintln!("Error sending queue message: {:?}", why);
                        }
                    }
                    
                    // Validate hex color format using regex.
                    let hex_regex = Regex::new(r"^#?([0-9a-fA-F]{3}){1,2}$").unwrap();
                    if !hex_regex.is_match(input_color) {
                        println!("Invalid hex color format: {}", input_color);
                        if let Err(why) = msg.channel_id.say(&ctx.http, "That doesn't look like a valid hex color or flavor. Please use formats like `#FF0000` or `FF0000` for colors, or specify a flavor like `latte`, `frappe`, `macchiato`, `mocha` with an image.").await {
                            eprintln!("Error sending message: {:?}", why);
                        }
                        return;
                    }

                    // Attempt to convert the color using the new helper function
                    match find_closest_catppuccin_hex(input_color, selected_flavor) {
                        Some((color_name, converted_hex)) => {
                            println!("Converted hex {} to Catppuccin color {} ({})", input_color, color_name, converted_hex);
                            let embed_color = u32::from_str_radix(&converted_hex, 16).unwrap_or(0x000000);

                            let original_color_display = if input_color.starts_with('#') {
                                input_color.to_string()
                            } else {
                                format!("#{}", input_color)
                            };
                            let converted_color_display = format!("#{}", converted_hex);

                            let embed = serenity::builder::CreateEmbed::default()
                                .title("Catppuccin Color Conversion")
                                        .description(format!("Original Color: `{}`", original_color_display))
                                        .color(embed_color)
                                        .field(
                                            "Closest Catppuccin Color",
                                            format!("**{}** (`{}`) (Flavor: {})", color_name.to_uppercase(), converted_color_display, selected_flavor.to_string().to_uppercase()),
                                            false,
                                        )
                                .field("\u{200b}", "**Color Swatch:** \u{2588}\u{2588}\u{2588}\u{2588}\u{2588}", false);

                            let builder = serenity::builder::CreateMessage::new().embed(embed);

                            if let Err(why) = msg.channel_id.send_message(&ctx.http, builder).await {
                                eprintln!("Error sending embed: {:?}", why);
                            }
                        }
                        None => {
                            println!("Error converting hex color: {}", input_color);
                            if let Err(why) = msg.channel_id.say(&ctx.http, "Error converting hex color. Please ensure it's a valid 3 or 6 digit hex code.").await {
                                eprintln!("Error sending error message: {:?}", why);
                            }
                        }
                    }
                    return; // Exit after handling hex conversion
                }
            }

            // --- Image Processing Logic ---
            if let Some(attachment) = msg.attachments.first() {
                // Acquire semaphore permit for processing
                let _permit = PROCESSING_SEMAPHORE.acquire().await.unwrap();
                
                // Send queue status if there are other tasks waiting
                let available_permits = PROCESSING_SEMAPHORE.available_permits();
                if available_permits == 0 {
                    if let Err(why) = msg.channel_id.say(&ctx.http, "⏳ Your request is queued. I'll process it as soon as possible!").await {
                        eprintln!("Error sending queue message: {:?}", why);
                    }
                }
                println!("Image attachment detected: {}", attachment.url);
                // Only process if it's an image
                let content_type_is_image = attachment.content_type.as_deref().map_or(false, |s| s.starts_with("image/"));
                if !content_type_is_image {
                    println!("Attachment is not an image: {:?}", attachment.content_type);
                    if let Err(why) = msg.channel_id.say(&ctx.http, "Please attach an image to catppuccinify it.").await {
                        eprintln!("Error sending message: {:?}", why);
                    }
                    return;
                }

                // Handle batch processing
                if batch_mode {
                    let all_attachments: Vec<_> = msg.attachments.iter()
                        .filter(|att| att.content_type.as_deref().map_or(false, |s| s.starts_with("image/")))
                        .collect();
                    
                    if all_attachments.is_empty() {
                        if let Err(why) = msg.channel_id.say(&ctx.http, "No valid images found for batch processing.").await {
                            eprintln!("Error sending message: {:?}", why);
                        }
                        return;
                    }
                    
                    let typing = msg.channel_id.start_typing(&ctx.http);
                    let mut processing_message_result = msg.channel_id.say(&ctx.http, &format!("🔄 Starting batch processing of {} images...", all_attachments.len())).await;
                    
                    let mut batch_attachments = Vec::new();
                    let reqwest_client = ReqwestClient::new();
                    
                    for (i, attachment) in all_attachments.iter().enumerate() {
                        println!("Processing batch image {}/{}", i + 1, all_attachments.len());
                        
                        // Update progress message
                        if let Ok(ref mut processing_msg) = processing_message_result {
                            let _ = update_progress_message(
                                &ctx,
                                msg.channel_id,
                                processing_msg,
                                &format!("📥 Downloading image {}/{}...", i + 1, all_attachments.len())
                            ).await;
                        }
                        
                        let image_bytes = match reqwest_client.get(&attachment.url).send().await {
                            Ok(response) => match response.bytes().await {
                                Ok(bytes) => bytes,
                                Err(e) => {
                                    eprintln!("Error reading image bytes: {:?}", e);
                                    continue;
                                }
                            },
                            Err(e) => {
                                eprintln!("Error downloading image: {:?}", e);
                                continue;
                            }
                        };
                        
                        let img = match ImageReader::new(Cursor::new(image_bytes))
                            .with_guessed_format()
                            .expect("Failed to guess image format")
                            .decode() {
                            Ok(img) => img,
                            Err(e) => {
                                eprintln!("Error decoding image: {:?}", e);
                                continue;
                            }
                        };
                        
                        let mut rgba_img = img.to_rgba8();
                        
                        // Update progress message
                        if let Ok(ref mut processing_msg) = processing_message_result {
                            let _ = update_progress_message(
                                &ctx,
                                msg.channel_id,
                                processing_msg,
                                &format!("🎨 Processing image {}/{} with {} flavor...", i + 1, all_attachments.len(), selected_flavor.to_string().to_uppercase())
                            ).await;
                        }
                        
                        let lut = generate_catppuccin_lut(selected_flavor, selected_algorithm);
                        apply_lut_to_image(&mut rgba_img, &lut);
                        
                        let mut output_buffer = Cursor::new(Vec::new());
                        let output_format = selected_format.unwrap_or_else(|| {
                            match attachment.content_type.as_deref() {
                                Some("image/png") => ImageFormat::Png,
                                Some("image/jpeg") => ImageFormat::Jpeg,
                                Some("image/gif") => ImageFormat::Gif,
                                Some("image/webp") => ImageFormat::WebP,
                                _ => ImageFormat::Png,
                            }
                        });
                        
                        let dynamic_img = match output_format {
                            ImageFormat::Jpeg => {
                                let rgb_img = image::DynamicImage::ImageRgba8(rgba_img).to_rgb8();
                                DynamicImage::ImageRgb8(rgb_img)
                            },
                            _ => DynamicImage::ImageRgba8(rgba_img),
                        };
                        
                        if let Err(e) = dynamic_img.write_to(&mut output_buffer, output_format) {
                            eprintln!("Error encoding batch image: {:?}", e);
                            continue;
                        }
                        
                        let filename = format!("batch_{}_{}.{}", 
                            i + 1, 
                            selected_flavor.to_string().to_lowercase(),
                            output_format.extensions_str().first().unwrap_or(&"png")
                        );
                        
                        let attachment_data = serenity::builder::CreateAttachment::bytes(
                            output_buffer.into_inner(), 
                            filename
                        );
                        batch_attachments.push(attachment_data);
                    }
                    
                    let _ = typing.stop();
                    
                    if !batch_attachments.is_empty() {
                        // Update final progress message
                        if let Ok(ref mut processing_msg) = processing_message_result {
                            let _ = send_success_message(
                                &ctx,
                                msg.channel_id,
                                processing_msg,
                                &format!("✅ **Batch Processing Complete!**\nProcessed {} images with {} flavor", 
                                    batch_attachments.len(), selected_flavor.to_string().to_uppercase())
                            ).await;
                        }
                        
                        let message_content = format!("**Batch Processing Complete**\nProcessed {} images with {} flavor", 
                            batch_attachments.len(), selected_flavor.to_string().to_uppercase());
                        let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                        
                        if let Err(why) = msg.channel_id.send_files(&ctx.http, batch_attachments, message_builder).await {
                            eprintln!("Error sending batch images: {:?}", why);
                        }
                    }
                    return;
                }

                // Send a "processing" message to the user
                let typing = msg.channel_id.start_typing(&ctx.http);
                let mut processing_message_result = msg.channel_id.say(&ctx.http, "🔄 Starting image processing...").await;
                println!("Downloading image from Discord...");

                let reqwest_client = ReqwestClient::new();
                
                // Update progress message
                if let Ok(ref mut processing_msg) = processing_message_result {
                    let _ = update_progress_message(
                        &ctx,
                        msg.channel_id,
                        processing_msg,
                        "📥 Downloading image from Discord..."
                    ).await;
                }
                
                let image_bytes = match reqwest_client.get(&attachment.url).send().await {
                    Ok(response) => {
                        match response.bytes().await {
                            Ok(bytes) => bytes,
                            Err(e) => {
                                eprintln!("Error reading image bytes: {:?}", e);
                                if let Err(why) = msg.channel_id.say(&ctx.http, "Failed to read image data.").await {
                                    eprintln!("Error sending message: {:?}", why);
                                }
                                let _ = typing.stop();
                                if let Ok(m) = processing_message_result { let _ = m.delete(&ctx.http).await; }
                                return;
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Error downloading image: {:?}", e);
                        if let Err(why) = msg.channel_id.say(&ctx.http, "Failed to download image from Discord.").await {
                            eprintln!("Error sending message: {:?}", why);
                        }
                        let _ = typing.stop();
                        if let Ok(m) = processing_message_result { let _ = m.delete(&ctx.http).await; }
                        return;
                    }
                };

                println!("Image downloaded. Decoding...");
                
                // Update progress message
                if let Ok(ref mut processing_msg) = processing_message_result {
                    let _ = update_progress_message(
                        &ctx,
                        msg.channel_id,
                        processing_msg,
                        "🔍 Decoding image..."
                    ).await;
                }

                // Load the image from bytes
                let img = match ImageReader::new(Cursor::new(image_bytes))
                    .with_guessed_format()
                    .expect("Failed to guess image format")
                    .decode() {
                    Ok(img) => img,
                    Err(e) => {
                        eprintln!("Error decoding image: {:?}", e);
                        if let Err(why) = msg.channel_id.say(&ctx.http, "Failed to decode the image. Is it a valid image file?").await {
                            eprintln!("Error sending message: {:?}", why);
                        }
                        let _ = typing.stop();
                        if let Ok(m) = processing_message_result { let _ = m.delete(&ctx.http).await; }
                        return;
                    }
                };

                println!("Image decoded. Converting to RGBA...");
                
                // Update progress message
                if let Ok(ref mut processing_msg) = processing_message_result {
                    let _ = update_progress_message(
                        &ctx,
                        msg.channel_id,
                        processing_msg,
                        "🔄 Converting image format..."
                    ).await;
                }

                // Convert to RGBA to ensure consistent pixel format for processing
                let mut rgba_img = img.to_rgba8();
                let (width, height) = rgba_img.dimensions();
                println!("Image dimensions: {}x{}", width, height);

                // Handle color statistics
                if show_stats {
                    let (dominant_colors, suggested_flavor) = analyze_image_colors(&rgba_img);
                    
                    let mut stats_message = format!("**Color Analysis Results**\n\n**Dominant Colors:**\n");
                    
                    for (i, (r, g, b, count)) in dominant_colors.iter().enumerate() {
                        let hex = format!("{:02X}{:02X}{:02X}", r, g, b);
                        let percentage = (*count as f32 / (width * height) as f32 * 100.0).round() as u32;
                        stats_message.push_str(&format!("{}. `#{}` (RGB: {},{},{}) - {}%\n", 
                            i + 1, hex, r, g, b, percentage));
                    }
                    
                    stats_message.push_str(&format!("\n**Suggested Flavor:** {}\n", suggested_flavor.to_string().to_uppercase()));
                    stats_message.push_str("\n*Based on average brightness of dominant colors*");
                    
                    if let Err(why) = msg.channel_id.say(&ctx.http, stats_message).await {
                        eprintln!("Error sending stats: {:?}", why);
                    }
                    return;
                }

                // Stop typing indicator
                let _ = typing.stop();

                if process_all_flavors {
                    // Process with all flavors
                    println!("Processing image with all flavors...");
                    
                    // Update progress message
                    if let Ok(ref mut processing_msg) = processing_message_result {
                        let _ = update_progress_message(
                            &ctx,
                            msg.channel_id,
                            processing_msg,
                            "🎨 Processing image with all flavors..."
                        ).await;
                    }
                    
                    let start_time = Instant::now();
                    
                    let flavors = [
                        (FlavorName::Latte, "latte"),
                        (FlavorName::Frappe, "frappe"), 
                        (FlavorName::Macchiato, "macchiato"),
                        (FlavorName::Mocha, "mocha")
                    ];
                    
                    let mut attachments = Vec::new();
                    
                    for (flavor, flavor_name) in flavors.iter() {
                        println!("Processing {} flavor...", flavor_name);
                        
                        // Update progress message
                        if let Ok(ref mut processing_msg) = processing_message_result {
                            let _ = update_progress_message(
                                &ctx,
                                msg.channel_id,
                                processing_msg,
                                &format!("🎨 Processing {} flavor...", flavor_name.to_uppercase())
                            ).await;
                        }
                        
                        // Create a copy of the image for this flavor
                        let mut flavor_img = rgba_img.clone();
                        
                        // Generate LUT for this flavor
                        let lut = generate_catppuccin_lut(*flavor, selected_algorithm);
                        
                        // Apply LUT to the image
                        apply_lut_to_image(&mut flavor_img, &lut);

                // Save the processed image to a buffer
                let mut output_buffer = Cursor::new(Vec::new());
                        let output_format = selected_format.unwrap_or_else(|| {
                            match attachment.content_type.as_deref() {
                                Some("image/png") => ImageFormat::Png,
                                Some("image/jpeg") => ImageFormat::Jpeg,
                                Some("image/gif") => ImageFormat::Gif,
                                Some("image/webp") => ImageFormat::WebP,
                                _ => ImageFormat::Png,
                            }
                        });

                        let dynamic_img = match output_format {
                            ImageFormat::Jpeg => {
                                let rgb_img = image::DynamicImage::ImageRgba8(flavor_img).to_rgb8();
                                DynamicImage::ImageRgb8(rgb_img)
                            },
                            _ => DynamicImage::ImageRgba8(flavor_img),
                        };

                        match dynamic_img.write_to(&mut output_buffer, output_format) {
                            Ok(_) => {},
                            Err(e) => {
                                eprintln!("Error encoding {} image: {:?}", flavor_name, e);
                                continue;
                            }
                        }

                        let filename = format!("catppuccinified_{}.{}", flavor_name, output_format.extensions_str().first().unwrap_or(&"png"));
                        let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename);
                        attachments.push(attachment_data);
                    }
                    
                    let elapsed = start_time.elapsed();
                    println!("All flavors processing complete in {:.2?}", elapsed);
                    
                    // Update final progress message
                    if let Ok(ref mut processing_msg) = processing_message_result {
                        let _ = send_success_message(
                            &ctx,
                            msg.channel_id,
                            processing_msg,
                            &format!("✅ **All Flavors Processing Complete!**\nProcessed in {:.2?}", elapsed)
                        ).await;
                    }
                    
                    if !attachments.is_empty() {
                        let message_content = format!("Here are your Catppuccinified images with all flavors! (Processed in {:.2?})", elapsed);
                        let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                        
                        if let Err(why) = msg.channel_id.send_files(&ctx.http, attachments, message_builder).await {
                            eprintln!("Error sending all flavors images: {:?}", why);
                            if let Err(why) = msg.channel_id.say(&ctx.http, "Failed to upload the processed images.").await {
                                eprintln!("Error sending message: {:?}", why);
                            }
                        } else {
                            println!("All flavors images sent successfully.");
                        }
                    }
                } else {
                    // Process with single flavor (existing logic)
                    // Start timing the processing
                    let start_time = Instant::now();
                    println!("Starting Catppuccinification of image...");
                    
                    // Update progress message
                    if let Ok(ref mut processing_msg) = processing_message_result {
                        let _ = update_progress_message(
                            &ctx,
                            msg.channel_id,
                            processing_msg,
                            &format!("🎨 Processing image with {} flavor...", selected_flavor.to_string().to_uppercase())
                        ).await;
                    }

                    // Generate LUT for the selected flavor using the specified algorithm
                    let lut = generate_catppuccin_lut(selected_flavor, selected_algorithm);

                    // Apply LUT to the image
                    apply_lut_to_image(&mut rgba_img, &lut);

                    let elapsed = start_time.elapsed();
                    println!("Image Catppuccinification complete in {:.2?}", elapsed);

                    // Handle comparison mode
                    if show_comparison {
                        // Update progress message
                        if let Ok(ref mut processing_msg) = processing_message_result {
                            let _ = update_progress_message(
                                &ctx,
                                msg.channel_id,
                                processing_msg,
                                "🔄 Creating before/after comparison..."
                            ).await;
                        }
                        
                        let original_img = img.to_rgba8();
                        let comparison_img = create_comparison_image(&original_img, &rgba_img);
                        
                        let mut output_buffer = Cursor::new(Vec::new());
                        let output_format = selected_format.unwrap_or(ImageFormat::Png);
                        
                        if let Err(e) = comparison_img.write_to(&mut output_buffer, output_format) {
                            eprintln!("Error encoding comparison image: {:?}", e);
                            if let Err(why) = msg.channel_id.say(&ctx.http, "Failed to create comparison image.").await {
                                eprintln!("Error sending message: {:?}", why);
                            }
                            return;
                        }
                        
                        let filename = format!("comparison_{}.{}", 
                            selected_flavor.to_string().to_lowercase(),
                            output_format.extensions_str().first().unwrap_or(&"png")
                        );
                        
                        let attachment_data = serenity::builder::CreateAttachment::bytes(
                            output_buffer.into_inner(), 
                            filename
                        );
                        
                        // Update final progress message
                        if let Ok(ref mut processing_msg) = processing_message_result {
                            let _ = send_success_message(
                                &ctx,
                                msg.channel_id,
                                processing_msg,
                                &format!("✅ **Comparison Complete!**\nLeft: Original | Right: {} flavor", 
                                    selected_flavor.to_string().to_uppercase())
                            ).await;
                        }
                        
                        let message_content = format!("**Before/After Comparison**\nLeft: Original | Right: {} flavor", 
                            selected_flavor.to_string().to_uppercase());
                        let message_builder = serenity::builder::CreateMessage::new().content(message_content);
                        
                        if let Err(why) = msg.channel_id.send_files(&ctx.http, vec![attachment_data], message_builder).await {
                            eprintln!("Error sending comparison image: {:?}", why);
                        }
                        return;
                    }

                    // Update progress message for encoding
                    if let Ok(ref mut processing_msg) = processing_message_result {
                        let _ = update_progress_message(
                            &ctx,
                            msg.channel_id,
                            processing_msg,
                            "💾 Encoding processed image..."
                        ).await;
                    }
                    
                    // Save the processed image to a buffer
                    let mut output_buffer = Cursor::new(Vec::new());
                    let output_format = selected_format.unwrap_or_else(|| {
                        match attachment.content_type.as_deref() {
                            Some("image/png") => ImageFormat::Png,
                            Some("image/jpeg") => ImageFormat::Jpeg,
                            Some("image/gif") => ImageFormat::Gif,
                            Some("image/webp") => ImageFormat::WebP,
                            _ => ImageFormat::Png,
                        }
                    });

                    println!("Encoding processed image...");
                    let dynamic_img = match output_format {
                        ImageFormat::Jpeg => {
                            let rgb_img = image::DynamicImage::ImageRgba8(rgba_img).to_rgb8();
                            DynamicImage::ImageRgb8(rgb_img)
                        },
                        _ => DynamicImage::ImageRgba8(rgba_img),
                    };

                    match dynamic_img.write_to(&mut output_buffer, output_format) {
                    Ok(_) => {},
                    Err(e) => {
                        eprintln!("Error encoding image: {:?}", e);
                        if let Err(why) = msg.channel_id.say(&ctx.http, "Failed to encode the processed image.").await {
                            eprintln!("Error sending message: {:?}", why);
                        }
                        return;
                    }
                }

                    println!("Sending processed image back to Discord...");
                    
                    // Update final progress message
                    if let Ok(ref mut processing_msg) = processing_message_result {
                        let _ = send_success_message(
                            &ctx,
                            msg.channel_id,
                            processing_msg,
                            &format!("✅ **Processing Complete!**\nFlavor: {} | Time: {:.2?}", 
                                selected_flavor.to_string().to_uppercase(), elapsed)
                        ).await;
                    }

                // Send the processed image back to Discord
                    let filename = format!("catppuccinified_{}.{}", 
                        selected_flavor.to_string().to_lowercase(), 
                        output_format.extensions_str().first().unwrap_or(&"png")
                    );
                let attachment_data = serenity::builder::CreateAttachment::bytes(output_buffer.into_inner(), filename.clone());

                    let mut message_content = format!("Here's your Catppuccinified image (Flavor: {})!", selected_flavor.to_string().to_uppercase());
                    
                    // Add quality and format info if specified
                    if let Some(quality) = selected_quality {
                        message_content.push_str(&format!(" Quality: {}", quality));
                    }
                    if let Some(format) = selected_format {
                        message_content.push_str(&format!(" Format: {}", format.extensions_str().first().unwrap_or(&"unknown")));
                    }
                    
                    let message_builder = serenity::builder::CreateMessage::new().content(message_content);

                    if let Err(why) = msg.channel_id.send_files(&ctx.http, vec![attachment_data], message_builder).await {
                    eprintln!("Error sending image: {:?}", why);
                    if let Err(why) = msg.channel_id.say(&ctx.http, "Failed to upload the processed image.").await {
                        eprintln!("Error sending message: {:?}", why);
                        }
                    } else {
                        println!("Processed image sent successfully.");
                    }
                }
            } else {
                // If no attachment and no hex code was provided after the command
                if parts.len() == 1 || (parts.len() == 2 && has_explicit_flavor_arg && parse_flavor(parts[1]).is_some()) {
                    println!("No hex code or image provided with command.");
                    if let Err(why) = msg.channel_id.say(&ctx.http, "Please provide a hex color code (e.g., `!cat #FF0000`) or attach an image with optional flavor and algorithm flags (e.g., `!cat mocha shepards`).\n\nAvailable flavors: latte, frappe, macchiato, mocha\nAvailable algorithms: shepards, gaussian, linear, sampling, nearest").await {
                        eprintln!("Error sending message: {:?}", why);
                    }
                }
            }
        }
    }

    // This event fires when the bot is ready.
    async fn ready(&self, _: Context, ready: serenity::model::gateway::Ready) {
        println!("{} is connected!", ready.user.name);
        println!("Bot is ready!");
    }
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv().ok(); // This line loads the .env file

    // Load the Discord bot token from environment variables.
    let token = env::var("DISCORD_BOT_TOKEN")
        .expect("Expected a Discord bot token in the environment variable DISCORD_BOT_TOKEN. Make sure you have a .env file with DISCORD_BOT_TOKEN=YOUR_TOKEN_HERE");

    // Create a new Discord client.
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::DIRECT_MESSAGES;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    // Start the Discord client.
    if let Err(why) = client.start().await {
        eprintln!("Client error: {:?}", why);
    }
}