use crate::error::{invalid, Result};
use crate::format::{CellEncoding, QuadQrCode, BLACK, BLUE, GREEN, RED, WHITE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    Screen,
    Print,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderStyle {
    Classic,
    Depth,
    Soft,
    Inset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub black: [u8; 3],
    pub white: [u8; 3],
    pub red: [u8; 3],
    pub green: [u8; 3],
    pub blue: [u8; 3],
}

impl Palette {
    pub const fn screen() -> Self {
        Self {
            black: [0, 0, 0],
            white: [255, 255, 255],
            red: [0xef, 0x23, 0x3c],
            green: [0x16, 0xa3, 0x4a],
            blue: [0x25, 0x63, 0xeb],
        }
    }

    pub const fn print() -> Self {
        Self {
            black: [0, 0, 0],
            white: [255, 255, 255],
            red: [0xd7, 0x19, 0x32],
            green: [0x08, 0x7f, 0x3e],
            blue: [0x17, 0x4e, 0xa6],
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::screen()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub module_size: u32,
    pub quiet_zone: u32,
    /// Optional palette override. `None` selects the reference Screen/Print palette from `mode`.
    pub palette: Option<Palette>,
    pub mode: RenderMode,
    pub style: RenderStyle,
    pub allow_styled_print: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            module_size: 8,
            quiet_zone: 4,
            palette: None,
            mode: RenderMode::Screen,
            style: RenderStyle::Classic,
            allow_styled_print: false,
        }
    }
}

impl RenderOptions {
    pub fn print() -> Self {
        Self {
            mode: RenderMode::Print,
            palette: None,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LogoOptions {
    /// Explicit matrix-relative logo ratio. Ignored when `auto_size` is true.
    pub size_ratio: f32,
    /// Match the JavaScript `size: "auto"` behavior using ECC/utilization.
    pub auto_size: bool,
    pub padding_modules: f32,
    pub radius_modules: f32,
    pub clear_background: bool,
    /// Optional clear-background color. `None` uses the active palette white.
    pub background_color: Option<[u8; 3]>,
}

impl Default for LogoOptions {
    fn default() -> Self {
        Self {
            size_ratio: 0.18,
            auto_size: false,
            padding_modules: 0.65,
            radius_modules: 0.8,
            clear_background: false,
            background_color: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PrintGuidance {
    pub physical_size_mm: f64,
    pub quiet_zone: u32,
    pub module_size_mm: f64,
    pub module_pixels_at_dpi: f64,
    pub dpi: f64,
    pub recommended_minimum_module_mm: f64,
    pub meets_recommended_module_size: bool,
    pub recommended_minimum_physical_size_mm: f64,
}

#[derive(Debug, Clone)]
pub struct LogoSearchResult {
    pub safe_size: f32,
    pub tested_max: f32,
    pub iterations: usize,
    pub empirical: bool,
}

pub fn estimate_safe_logo_size(code: &QuadQrCode, mode: RenderMode, clear_background: bool) -> f32 {
    let base = match code.ecc_level {
        crate::format::EccLevel::L => 0.10,
        crate::format::EccLevel::M => 0.13,
        crate::format::EccLevel::Q => 0.16,
        crate::format::EccLevel::H => 0.19,
    };
    let utilization_penalty = if code.utilization <= 0.55 {
        1.0
    } else {
        (1.0 - (code.utilization - 0.55) * 0.56).max(0.72)
    };
    let clear_penalty = if clear_background { 0.90 } else { 1.0 };
    let print_penalty = if mode == RenderMode::Print { 0.88 } else { 1.0 };
    let version_penalty = if code.version == 1 { 0.88 } else { 1.0 };
    (base * utilization_penalty * clear_penalty * print_penalty * version_penalty).clamp(0.07, 0.22)
        as f32
}

fn resolved_logo_ratio(code: &QuadQrCode, render: RenderOptions, logo: LogoOptions) -> f32 {
    if logo.auto_size {
        estimate_safe_logo_size(code, render.mode, logo.clear_background)
    } else {
        logo.size_ratio.clamp(0.05, 0.30)
    }
}

pub fn get_print_guidance(
    code: &QuadQrCode,
    quiet_zone: u32,
    physical_size_mm: f64,
    dpi: f64,
    minimum_module_mm: f64,
) -> PrintGuidance {
    let quiet_zone = quiet_zone.max(4);
    let physical_size_mm = if physical_size_mm.is_finite() && physical_size_mm > 0.0 {
        physical_size_mm
    } else {
        35.0
    };
    let dpi = if dpi.is_finite() && dpi > 0.0 {
        dpi
    } else {
        300.0
    };
    let minimum_module_mm = if minimum_module_mm.is_finite() && minimum_module_mm > 0.0 {
        minimum_module_mm
    } else {
        0.40
    };
    let total_modules = code.size as f64 + quiet_zone as f64 * 2.0;
    let module_size_mm = physical_size_mm / total_modules;
    let module_pixels_at_dpi = module_size_mm / 25.4 * dpi;
    PrintGuidance {
        physical_size_mm,
        quiet_zone,
        module_size_mm,
        module_pixels_at_dpi,
        dpi,
        recommended_minimum_module_mm: minimum_module_mm,
        meets_recommended_module_size: module_size_mm >= minimum_module_mm,
        recommended_minimum_physical_size_mm: total_modules * minimum_module_mm,
    }
}

fn effective_palette(options: RenderOptions) -> Palette {
    options.palette.unwrap_or_else(|| match options.mode {
        RenderMode::Screen => Palette::screen(),
        RenderMode::Print => Palette::print(),
    })
}

fn effective_style(options: RenderOptions) -> RenderStyle {
    if options.mode == RenderMode::Print && !options.allow_styled_print {
        RenderStyle::Classic
    } else {
        options.style
    }
}

fn rgb_for(cell: i16, palette: Palette) -> Result<[u8; 3]> {
    match cell {
        BLACK => Ok(palette.black),
        RED => Ok(palette.red),
        GREEN => Ok(palette.green),
        BLUE => Ok(palette.blue),
        WHITE => Ok(palette.white),
        _ => Err(invalid(format!("invalid RGBW cell {cell}"))),
    }
}

fn triangle_colors(value: i16, palette: Palette) -> Result<([u8; 3], [u8; 3])> {
    if !(0..=15).contains(&value) {
        return Err(invalid(format!("invalid Triangle16 cell {value}")));
    }
    Ok((
        rgb_for((value >> 2) & 3, palette)?,
        rgb_for(value & 3, palette)?,
    ))
}

fn mix(a: [u8; 3], b: [u8; 3], amount: f64) -> [u8; 3] {
    let t = amount.clamp(0.0, 1.0);
    [
        (a[0] as f64 + (b[0] as f64 - a[0] as f64) * t).round() as u8,
        (a[1] as f64 + (b[1] as f64 - a[1] as f64) * t).round() as u8,
        (a[2] as f64 + (b[2] as f64 - a[2] as f64) * t).round() as u8,
    ]
}

fn render_noise(row: usize, col: usize, cell: i16) -> f64 {
    let mut value = (row as u32 + 1).wrapping_mul(0x9e3779b1)
        ^ (col as u32 + 1).wrapping_mul(0x85ebca6b)
        ^ ((cell + 2) as u32).wrapping_mul(0xc2b2ae35);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846ca68b);
    value ^= value >> 16;
    value as f64 / u32::MAX as f64
}

fn depth_opacity(row: usize, col: usize, cell: i16) -> f64 {
    let value = render_noise(row, col, cell);
    if value < 0.16 {
        0.80
    } else if value < 0.36 {
        0.87
    } else if value < 0.56 {
        0.94
    } else {
        1.0
    }
}

fn visible_cell(cell: i16) -> i16 {
    if cell > 3 {
        let first = (cell >> 2) & 3;
        let second = cell & 3;
        if first == second {
            first
        } else {
            WHITE
        }
    } else {
        cell
    }
}

#[cfg(feature = "image")]
fn fill_rect(image: &mut image::RgbaImage, x: u32, y: u32, width: u32, height: u32, rgb: [u8; 3]) {
    let pixel = image::Rgba([rgb[0], rgb[1], rgb[2], 255]);
    let max_x = x.saturating_add(width).min(image.width());
    let max_y = y.saturating_add(height).min(image.height());
    for yy in y..max_y {
        for xx in x..max_x {
            image.put_pixel(xx, yy, pixel);
        }
    }
}

#[cfg(feature = "image")]
fn fill_rounded(
    image: &mut image::RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    radius: u32,
    rgb: [u8; 3],
) {
    if width == 0 || height == 0 {
        return;
    }

    let r = radius.min(width / 2).min(height / 2) as i64;
    let left_center = x as i64 + r;
    let right_center = (x + width - 1) as i64 - r;
    let top_center = y as i64 + r;
    let bottom_center = (y + height - 1) as i64 - r;
    let pixel = image::Rgba([rgb[0], rgb[1], rgb[2], 255]);

    let max_x = x.saturating_add(width).min(image.width());
    let max_y = y.saturating_add(height).min(image.height());
    for yy in y..max_y {
        for xx in x..max_x {
            let xi = xx as i64;
            let yi = yy as i64;
            let dx = if xi < left_center {
                left_center - xi
            } else if xi > right_center {
                xi - right_center
            } else {
                0
            };
            let dy = if yi < top_center {
                top_center - yi
            } else if yi > bottom_center {
                yi - bottom_center
            } else {
                0
            };
            if dx * dx + dy * dy <= r * r {
                image.put_pixel(xx, yy, pixel);
            }
        }
    }
}

#[cfg(feature = "image")]
pub fn render_image(code: &QuadQrCode, options: RenderOptions) -> Result<image::RgbaImage> {
    use image::{Rgba, RgbaImage};

    if options.module_size == 0 {
        return Err(invalid("module_size must be positive"));
    }

    let palette = effective_palette(options);
    let style = effective_style(options);
    let total_modules = code.size as u32 + options.quiet_zone * 2;
    let pixels = total_modules * options.module_size;
    let mut output = RgbaImage::from_pixel(
        pixels,
        pixels,
        Rgba([palette.white[0], palette.white[1], palette.white[2], 255]),
    );
    let layout = crate::format::layout_for_version(code.version)?;

    for row in 0..code.size {
        for col in 0..code.size {
            let cell = code.matrix[row][col];
            let structural = layout.reserved[row][col] || cell == BLACK;
            let x = (col as u32 + options.quiet_zone) * options.module_size;
            let y = (row as u32 + options.quiet_zone) * options.module_size;

            if code.cell_encoding == CellEncoding::Triangle16 && !structural {
                let (first, second) = triangle_colors(cell, palette)?;
                let side = options.module_size;
                for py in 0..side {
                    for px in 0..side {
                        let upper_left = ((px as f64 + 0.5) / side as f64
                            + (py as f64 + 0.5) / side as f64)
                            <= 1.0;
                        let rgb = if upper_left { first } else { second };
                        output.put_pixel(x + px, y + py, Rgba([rgb[0], rgb[1], rgb[2], 255]));
                    }
                }
                continue;
            }

            let rgb = rgb_for(visible_cell(cell), palette)?;
            if style == RenderStyle::Classic || structural {
                fill_rect(
                    &mut output,
                    x,
                    y,
                    options.module_size,
                    options.module_size,
                    rgb,
                );
                continue;
            }

            match style {
                RenderStyle::Classic => {}
                RenderStyle::Depth => {
                    let base = mix(palette.white, rgb, depth_opacity(row, col, cell));
                    fill_rect(
                        &mut output,
                        x,
                        y,
                        options.module_size,
                        options.module_size,
                        base,
                    );
                    if options.module_size >= 6 {
                        let edge = (options.module_size as f64 * 0.08).floor().max(1.0) as u32;
                        let highlight = mix(base, palette.white, 0.17);
                        let shadow = mix(base, palette.black, 0.12);
                        fill_rect(&mut output, x, y, options.module_size, edge, highlight);
                        fill_rect(&mut output, x, y, edge, options.module_size, highlight);
                        fill_rect(
                            &mut output,
                            x,
                            y + options.module_size - edge,
                            options.module_size,
                            edge,
                            shadow,
                        );
                        fill_rect(
                            &mut output,
                            x + options.module_size - edge,
                            y,
                            edge,
                            options.module_size,
                            shadow,
                        );
                    }
                }
                RenderStyle::Soft => {
                    let inset = if options.module_size >= 6 {
                        (options.module_size as f64 * 0.07).floor().max(1.0) as u32
                    } else {
                        0
                    };
                    let side = options.module_size.saturating_sub(inset * 2).max(1);
                    let radius = (options.module_size as f64 * 0.20).floor().max(1.0) as u32;
                    fill_rounded(&mut output, x + inset, y + inset, side, side, radius, rgb);
                }
                RenderStyle::Inset => {
                    fill_rect(
                        &mut output,
                        x,
                        y,
                        options.module_size,
                        options.module_size,
                        rgb,
                    );
                    if visible_cell(cell) != WHITE {
                        let edge = ((options.module_size as f64 * 0.10).floor() as u32)
                            .min((options.module_size as f64 * 0.16).floor() as u32);
                        if edge > 0 {
                            let shadow = mix(rgb, palette.black, 0.10);
                            let highlight = mix(rgb, palette.white, 0.12);
                            fill_rect(&mut output, x, y, options.module_size, edge, shadow);
                            fill_rect(&mut output, x, y, edge, options.module_size, shadow);
                            fill_rect(
                                &mut output,
                                x,
                                y + options.module_size - edge,
                                options.module_size,
                                edge,
                                highlight,
                            );
                            fill_rect(
                                &mut output,
                                x + options.module_size - edge,
                                y,
                                edge,
                                options.module_size,
                                highlight,
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(output)
}

#[cfg(feature = "image")]
pub fn render_image_with_logo(
    code: &QuadQrCode,
    options: RenderOptions,
    logo: &image::DynamicImage,
    logo_options: LogoOptions,
) -> Result<image::RgbaImage> {
    use image::{imageops::FilterType, Rgba};

    let mut output = render_image(code, options)?;
    let ratio = resolved_logo_ratio(code, options, logo_options);
    let code_pixels = code.size as f32 * options.module_size as f32;
    let logo_pixels = (code_pixels * ratio).round().max(1.0) as u32;
    let padding =
        (logo_options.padding_modules.max(0.0) * options.module_size as f32).round() as u32;
    let background_size = logo_pixels + padding * 2;
    let center_x = output.width() / 2;
    let center_y = output.height() / 2;

    if logo_options.clear_background {
        let x = center_x.saturating_sub(background_size / 2);
        let y = center_y.saturating_sub(background_size / 2);
        fill_rounded(
            &mut output,
            x,
            y,
            background_size,
            background_size,
            (options.module_size as f32 * logo_options.radius_modules.max(0.0)).round() as u32,
            logo_options
                .background_color
                .unwrap_or(effective_palette(options).white),
        );
    }

    let rgba = logo.to_rgba8();
    let scale =
        (logo_pixels as f64 / rgba.width() as f64).min(logo_pixels as f64 / rgba.height() as f64);
    let resized_width = (rgba.width() as f64 * scale).round().max(1.0) as u32;
    let resized_height = (rgba.height() as f64 * scale).round().max(1.0) as u32;
    let resized =
        image::imageops::resize(&rgba, resized_width, resized_height, FilterType::Lanczos3);
    let start_x = center_x.saturating_sub(resized_width / 2);
    let start_y = center_y.saturating_sub(resized_height / 2);

    for (x, y, pixel) in resized.enumerate_pixels() {
        let alpha = pixel[3] as f32 / 255.0;
        if alpha <= 0.0 {
            continue;
        }
        let target_x = start_x + x;
        let target_y = start_y + y;
        if target_x >= output.width() || target_y >= output.height() {
            continue;
        }
        let destination = output.get_pixel(target_x, target_y).0;
        let blend = |source: u8, dest: u8| {
            (source as f32 * alpha + dest as f32 * (1.0 - alpha)).round() as u8
        };
        output.put_pixel(
            target_x,
            target_y,
            Rgba([
                blend(pixel[0], destination[0]),
                blend(pixel[1], destination[1]),
                blend(pixel[2], destination[2]),
                255,
            ]),
        );
    }

    Ok(output)
}

pub fn render_svg_with_logo_href(
    code: &QuadQrCode,
    options: RenderOptions,
    logo_href: &str,
    logo_options: LogoOptions,
) -> Result<String> {
    let mut svg = render_svg(code, options)?;
    let ratio = resolved_logo_ratio(code, options, logo_options);
    let total = (code.size as u32 + options.quiet_zone * 2) as f32 * options.module_size as f32;
    let code_pixels = code.size as f32 * options.module_size as f32;
    let logo_pixels = (code_pixels * ratio).max(1.0);
    let padding = logo_options.padding_modules.max(0.0) * options.module_size as f32;
    let background_size = logo_pixels + padding * 2.0;
    let center = total / 2.0;
    let logo_x = center - logo_pixels / 2.0;
    let logo_y = center - logo_pixels / 2.0;
    let mut overlay = String::new();
    if logo_options.clear_background {
        let x = center - background_size / 2.0;
        let y = center - background_size / 2.0;
        let radius = logo_options.radius_modules.max(0.0) * options.module_size as f32;
        let background = logo_options
            .background_color
            .unwrap_or(effective_palette(options).white);
        overlay.push_str(&format!(
            r#"<rect x="{x}" y="{y}" width="{background_size}" height="{background_size}" rx="{radius}" fill="{}"/>"#,
            hex(background)
        ));
    }
    let escaped_href = logo_href
        .replace('&', "&amp;")
        .replace('\"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    overlay.push_str(&format!(
        r#"<image href="{escaped_href}" x="{logo_x}" y="{logo_y}" width="{logo_pixels}" height="{logo_pixels}" preserveAspectRatio="xMidYMid meet"/>"#
    ));
    if let Some(index) = svg.rfind("</svg>") {
        svg.insert_str(index, &overlay);
    }
    Ok(svg)
}

#[cfg(feature = "image")]
pub fn find_max_safe_logo_size(
    code: &QuadQrCode,
    render_options: RenderOptions,
    logo: &image::DynamicImage,
    mut logo_options: LogoOptions,
    min_size: f32,
    max_size: f32,
    iterations: usize,
) -> Result<LogoSearchResult> {
    let mut low = min_size.clamp(0.05, 0.30);
    let mut high = max_size.clamp(low, 0.30);
    let tested_max = high;
    let iterations = iterations.clamp(3, 10);
    let mut best = 0.0f32;
    logo_options.auto_size = false;
    for _ in 0..iterations {
        let probe = (low + high) / 2.0;
        logo_options.size_ratio = probe;
        let image = render_image_with_logo(code, render_options, logo, logo_options)?;
        let scan = crate::vision::scan_rgba(
            &image,
            &crate::vision::ScanOptions {
                min_version: code.version,
                max_version: code.version,
                ..crate::vision::ScanOptions::default()
            },
        );
        let ok = scan
            .map(|result| result.decoded.crc32 == code.crc32)
            .unwrap_or(false);
        if ok {
            best = probe;
            low = probe;
        } else {
            high = probe;
        }
    }
    Ok(LogoSearchResult {
        safe_size: if best > 0.0 {
            best
        } else {
            estimate_safe_logo_size(code, render_options.mode, logo_options.clear_background)
        },
        tested_max,
        iterations,
        empirical: best > 0.0,
    })
}

#[cfg(feature = "image")]
pub fn to_png(code: &QuadQrCode, options: RenderOptions) -> Result<Vec<u8>> {
    let image = render_image(code, options)?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image).write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(cursor.into_inner())
}

#[cfg(feature = "image")]
pub fn to_png_with_logo(
    code: &QuadQrCode,
    options: RenderOptions,
    logo: &image::DynamicImage,
    logo_options: LogoOptions,
) -> Result<Vec<u8>> {
    let image = render_image_with_logo(code, options, logo, logo_options)?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image).write_to(&mut cursor, image::ImageFormat::Png)?;
    Ok(cursor.into_inner())
}

pub fn save_svg(
    code: &QuadQrCode,
    path: impl AsRef<std::path::Path>,
    options: RenderOptions,
) -> Result<()> {
    std::fs::write(path, render_svg(code, options)?)?;
    Ok(())
}

pub fn save_svg_with_logo_href(
    code: &QuadQrCode,
    path: impl AsRef<std::path::Path>,
    options: RenderOptions,
    logo_href: &str,
    logo_options: LogoOptions,
) -> Result<()> {
    std::fs::write(
        path,
        render_svg_with_logo_href(code, options, logo_href, logo_options)?,
    )?;
    Ok(())
}

#[cfg(feature = "image")]
pub fn save_png(
    code: &QuadQrCode,
    path: impl AsRef<std::path::Path>,
    options: RenderOptions,
) -> Result<()> {
    render_image(code, options)?.save(path)?;
    Ok(())
}

fn hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

pub fn render_svg(code: &QuadQrCode, options: RenderOptions) -> Result<String> {
    if options.module_size == 0 {
        return Err(invalid("module_size must be positive"));
    }

    let palette = effective_palette(options);
    let style = effective_style(options);
    let total = (code.size as u32 + options.quiet_zone * 2) * options.module_size;
    let mut output = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{total}" height="{total}" viewBox="0 0 {total} {total}" shape-rendering="crispEdges"><rect width="100%" height="100%" fill="{}"/>"#,
        hex(palette.white)
    );
    let layout = crate::format::layout_for_version(code.version)?;

    for row in 0..code.size {
        for col in 0..code.size {
            let cell = code.matrix[row][col];
            let structural = layout.reserved[row][col] || cell == BLACK;
            let x = (col as u32 + options.quiet_zone) * options.module_size;
            let y = (row as u32 + options.quiet_zone) * options.module_size;

            if code.cell_encoding == CellEncoding::Triangle16 && !structural {
                let (first, second) = triangle_colors(cell, palette)?;
                let side = options.module_size;
                output.push_str(&format!(
                    r#"<path d="M{x},{y} h{side} L{x},{} Z" fill="{}"/>"#,
                    y + side,
                    hex(first)
                ));
                output.push_str(&format!(
                    r#"<path d="M{},{} H{x} L{},{} Z" fill="{}"/>"#,
                    x + side,
                    y + side,
                    x + side,
                    y,
                    hex(second)
                ));
                continue;
            }

            let rgb = rgb_for(visible_cell(cell), palette)?;
            if style == RenderStyle::Classic || structural {
                if rgb != palette.white {
                    output.push_str(&format!(
                        r#"<rect x="{x}" y="{y}" width="{}" height="{}" fill="{}"/>"#,
                        options.module_size,
                        options.module_size,
                        hex(rgb)
                    ));
                }
                continue;
            }

            match style {
                RenderStyle::Classic => {}
                RenderStyle::Depth => {
                    let base = mix(palette.white, rgb, depth_opacity(row, col, cell));
                    output.push_str(&format!(
                        r#"<rect x="{x}" y="{y}" width="{}" height="{}" fill="{}"/>"#,
                        options.module_size,
                        options.module_size,
                        hex(base)
                    ));
                }
                RenderStyle::Soft => {
                    let inset = if options.module_size >= 6 {
                        (options.module_size as f64 * 0.07).floor().max(1.0) as u32
                    } else {
                        0
                    };
                    let side = options.module_size.saturating_sub(inset * 2).max(1);
                    let radius = (options.module_size as f64 * 0.20).floor().max(1.0) as u32;
                    output.push_str(&format!(
                        r#"<rect x="{}" y="{}" width="{side}" height="{side}" rx="{radius}" fill="{}"/>"#,
                        x + inset,
                        y + inset,
                        hex(rgb)
                    ));
                }
                RenderStyle::Inset => {
                    output.push_str(&format!(
                        r#"<rect x="{x}" y="{y}" width="{}" height="{}" fill="{}"/>"#,
                        options.module_size,
                        options.module_size,
                        hex(rgb)
                    ));
                }
            }
        }
    }

    output.push_str("</svg>");
    Ok(output)
}
