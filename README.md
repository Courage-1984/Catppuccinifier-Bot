# Catppuccinifier Discord Bot

A fast, beautiful, and customizable Discord bot that transforms images with the Catppuccin color palette! Supports advanced palette mapping, multiple flavors, algorithms, formats, and more. Built in Rust using the Serenity framework.

---

## ✨ Features

- Catppuccinify any image with your favorite flavor (Latte, Frappe, Macchiato, Mocha)
- Multiple palette mapping algorithms (Shepard's, Gaussian, Nearest Neighbor, Hald, etc.)
- Supports PNG, JPG, WEBP, GIF (animated), BMP
- Batch processing: process multiple images at once
- Animated GIF support: all frames are processed
- Palette previews for each flavor and all flavors
- Hex color conversion to closest Catppuccin color
- Color statistics: dominant colors and suggested flavor
- Before/after comparison images
- Quality and export format options
- Process images from Discord attachments or direct URLs
- Random color and palette preview commands
- Queueing and cancellation for long-running jobs
- Ephemeral typing indicators for user feedback
- Dynamic list of available flavors, algorithms, and formats
- Secure: Enforces max image size and dimensions
- Detailed error handling and logging

---

## 🚀 Installation & Setup

### 1. Clone the repository

```sh
git clone https://github.com/yourusername/Catppuccinifier_bot.git
cd Catppuccinifier_bot
```

### 2. Install Rust (if not already)

- [Install Rust](https://rustup.rs/)

### 3. Set up environment variables

Create a `.env` file in the project root:

```
DISCORD_BOT_TOKEN=your_discord_bot_token_here
```

### 4. Build and run the bot

```sh
cargo run --release
```

---

## ⚙️ Environment Variables

- `DISCORD_BOT_TOKEN` — Your Discord bot token (required)

---

## 📝 Usage Instructions

Invite the bot to your server and use the `!cat` command. The bot responds to commands in any channel it can read.

### Basic Image Processing

- Attach an image and type:
  ```
  !cat
  ```
- Or specify a flavor:
  ```
  !cat mocha
  ```
- Or process an image from a URL:
  ```
  !cat https://example.com/image.png
  ```

### Advanced Usage & Command Variants

- **Color Schemes:**
  - Analyze and preview color schemes based on the dominant color of an image:
    ```
    !cat scheme complementary [image]
    !cat scheme analogous [image]
    !cat scheme triadic [image]
    !cat scheme monochromatic [image]
    ```
  - Supported types: `complementary`, `analogous`, `triadic`, `monochromatic`.
- **Batch Processing:**
  - Attach multiple images and type:
    ```
    !cat batch
    ```
  - Or just attach multiple images with `!cat`
- **All Flavors:**
  - Process with all flavors at once:
    ```
    !cat all [image]
    ```
- **Palette Previews:**
  - Show a flavor's palette:
    ```
    !cat palette latte
    ```
  - Show all palettes:
    ```
    !cat palette all
    ```
  - Show a random palette:
    ```
    !cat random palette
    ```
- **Hex Color Conversion:**
  - Convert a hex color to the closest Catppuccin color:
    ```
    !cat #FF0000
    !cat mocha #00FF00
    ```
- **Before/After Comparison:**
  - Compare original and processed image:
    ```
    !cat compare [image]
    ```
- **Color Statistics:**
  - Show dominant colors and suggested flavor:
    ```
    !cat stats [image]
    ```
- **Quality, Algorithm, and Format:**
  - Specify quality:
    ```
    !cat mocha high [image]
    !cat frappe fast [image]
    ```
  - Specify algorithm:
    ```
    !cat macchiato gaussian [image]
    !cat mocha shepards [image]
    ```
  - Specify export format:
    ```
    !cat latte png [image]
    !cat frappe webp [image]
    ```
- **List Options:**
  - List all flavors, algorithms, and formats:
    ```
    !cat list
    ```
- **Cancel:**
  - Cancel your current job:
    ```
    !cat cancel
    ```
- **Random Color:**
  - Get a random Catppuccin color:
    ```
    !cat random
    ```
- **Help:**
  - Show help message:
    ```
    !cat help
    !cat -h
    ```

### **Advanced Color Analysis**

- **Color Palette Extraction**: `!cat extract [image]` - Extract the actual color palette from an image
- **Color Harmony Analysis**: `!cat harmony [image]` - Show complementary, analogous, triadic colors
- **Color Blindness Simulation**: `!cat simulate [type] [image]` - Show how image looks to colorblind users (`protanopia`, `deuteranopia`, `tritanopia`)
- **Color Temperature Analysis**: `!cat temperature [image]` - Analyze warm vs cool colors

### Advanced Usage & Command Variants

- **Texture Overlays:**
  - Overlay Catppuccin-themed textures on images:
    ```
    !cat texture dots [image]
    !cat texture stripes [image]
    ```
  - Supported types: `dots`, `stripes`.

### All Commands Table

| Command                             | Description                                                              |
| ----------------------------------- | ------------------------------------------------------------------------ | ----------------------- |
| `!cat [image]`                      | Process image with default Latte flavor                                  |
| `!cat [flavor] [image]`             | Process image with specific flavor                                       |
| `!cat [flavor] [algorithm] [image]` | Use a specific algorithm                                                 |
| `!cat [flavor] [quality] [image]`   | Use a quality preset (fast, normal, high)                                |
| `!cat [flavor] [format] [image]`    | Export as PNG, JPG, WEBP, GIF                                            |
| `!cat all [image]`                  | Process with all flavors                                                 |
| `!cat batch [images]`               | Batch process multiple images                                            |
| `!cat palette [flavor               | all]`                                                                    | Show palette preview(s) |
| `!cat compare [image]`              | Before/after comparison                                                  |
| `!cat stats [image]`                | Show dominant colors and suggest flavor                                  |
| `!cat #HEX`                         | Convert hex color to Catppuccin                                          |
| `!cat [flavor] #HEX`                | Convert hex color for a specific flavor                                  |
| `!cat list`                         | List all flavors, algorithms, formats                                    |
| `!cat cancel`                       | Cancel your current job                                                  |
| `!cat random`                       | Get a random Catppuccin color                                            |
| `!cat random palette`               | Get a random palette preview                                             |
| `!cat help`                         | Show help message                                                        |
| `!cat gradient [colors]`            | Generate a gradient from Catppuccin color names or hex codes             |
| `!cat scheme [type] [image]`        | Preview color schemes (complementary, analogous, triadic, monochromatic) |
| `!cat animate [effect] [image]`     | Add animation effects (e.g., fade) to images as GIF                      |
| `!cat texture [type] [image]`       | Overlay Catppuccin-themed textures (dots, stripes) on images             |

---

## 🧩 Available Options

### Flavors

- `latte` — Light, warm theme
- `frappe` — Medium, balanced theme
- `macchiato` — Dark, rich theme
- `mocha` — Darkest, deep theme

### Algorithms

- `shepards-method` — Best quality (default)
- `gaussian-rbf` — Smooth gradients
- `linear-rbf` — Fast processing
- `gaussian-sampling` — High quality, slower
- `nearest-neighbor` — Fastest, basic
- `hald` — Hald CLUT method
- `euclide` — Euclidean distance
- `mean` — Mean-based mapping
- `std` — Standard deviation method

### Quality Levels

- `fast` — Nearest neighbor (fastest)
- `normal` — Shepard's method (balanced)
- `high` — Gaussian sampling (best quality)

### Export Formats

- `png` — Lossless, supports transparency
- `jpg` — Compressed, smaller files
- `webp` — Modern, good compression
- `gif` — Animated images
- `bmp` — Bitmap

---

## 🛡️ Security & Limits

- **Max file size:** 8 MB
- **Max dimensions:** 4096 x 4096 pixels
- **Input validation:**
  - Validates hex color, image format, URL length, etc.
  - Only processes valid image attachments or direct image URLs
- **Error handling:**
  - User-friendly error messages for all failure cases (invalid input, download errors, decode errors, etc.)
- **Job cancellation:**
  - Users can cancel their own running jobs with `!cat cancel`
- **Concurrency:**
  - Limits concurrent image processing jobs to avoid overload

---

## 📄 Logging

- Errors and important events are logged to `catppuccin_bot.log` in the project root

---

## 🧑‍💻 Developer Notes

### Code Structure

- `src/main.rs`: Bot entry point, command framework, top-level error handling
- `src/commands.rs`: Discord event handler, command parsing, and dispatch
- `src/image_processing.rs`: Image and GIF processing, palette mapping, LUT generation, color analysis
- `src/palette.rs`: Palette preview image generation
- `src/utils.rs`: Helpers for parsing, color conversion, and constants

### Testing

- Unit tests for palette preview, LUT generation, GIF processing, and utility functions
- Run all tests:
  ```sh
  cargo test
  ```

### Extending the Bot

- Add new flavors, algorithms, or formats by updating `utils.rs` and `image_processing.rs`
- Add new commands or features in `commands.rs` and register in `main.rs`
- Palette and color logic is modular for easy extension

---

## 🤝 Contributing

Pull requests and issues are welcome! Please open an issue to discuss major changes first. See developer notes above for code structure and extension points.

---

## 📜 License

MIT License. See [LICENSE](LICENSE) for details.

## 📝 Help Command Output

The `!cat -h` or `!cat help` command now includes:

- `!cat extract [image]` — Extract the actual color palette from an image
- `!cat harmony [image]` — Show complementary, analogous, triadic colors for the dominant color
- `!cat simulate [type] [image]` — Simulate color blindness (`protanopia`, `deuteranopia`, `tritanopia`)
- `!cat temperature [image]` — Analyze and report the proportion of warm vs cool colors
- `!cat gradient [colors]` — Generate a gradient from Catppuccin color names or hex codes
- `!cat scheme [type] [image]` — Preview color schemes (complementary, analogous, triadic, monochromatic)
- `!cat animate [effect] [image]` — Add animation effects (e.g., fade) to images as GIF
- `!cat texture [type] [image]` — Overlay Catppuccin-themed textures (dots, stripes) on images

(These are in addition to all previously documented features.)

### Example Help Output

```
!cat extract [image]      - Extract the actual color palette from an image
!cat harmony [image]      - Show complementary, analogous, triadic colors for the dominant color
!cat simulate [type] [image] - Simulate color blindness (protanopia, deuteranopia, tritanopia)
!cat temperature [image]  - Analyze and report the proportion of warm vs cool colors
!cat gradient [colors]    - Generate a gradient from Catppuccin color names or hex codes
!cat scheme [type] [image] - Preview color schemes (complementary, analogous, triadic, monochromatic)
!cat animate [effect] [image] - Add animation effects (e.g., fade) to images as GIF
!cat texture [type] [image] - Overlay Catppuccin-themed textures (dots, stripes) on images
```
