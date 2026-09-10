use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use anyhow::{Context, Result};
use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::io::Cursor;
use std::sync::Arc;

pub const CINZEL: &[u8] = include_bytes!("../assets/fonts/Cinzel-Bold.ttf");
pub const RAJDHANI: &[u8] = include_bytes!("../assets/fonts/Rajdhani-Medium.ttf");

pub fn apply_egui_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "rajdhani".to_owned(),
        Arc::new(FontData::from_static(RAJDHANI)),
    );
    fonts.font_data.insert(
        "cinzel".to_owned(),
        Arc::new(FontData::from_static(CINZEL)),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "rajdhani".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push("rajdhani".to_owned());
    fonts.families.insert(
        FontFamily::Name("cinzel".into()),
        vec!["cinzel".to_owned(), "rajdhani".to_owned()],
    );
    ctx.set_fonts(fonts);
}

pub fn heading_family() -> FontFamily {
    FontFamily::Name("cinzel".into())
}

pub fn font_has_char(bytes: &[u8], ch: char) -> bool {
    FontRef::try_from_slice(bytes)
        .map(|f| f.glyph_id(ch).0 != 0)
        .unwrap_or(false)
}

const PDF_BG_JPEG: &[u8] = include_bytes!("../assets/bg/pdf-bg.jpg");

fn heading_metal(w: u32, h: u32) -> image::RgbImage {
    let w = w.max(8);
    let h = h.max(8);
    if let Ok(src) = image::load_from_memory(PDF_BG_JPEG) {
        let src = src.to_rgb8();
        let sw = src.width().max(1);
        let sh = src.height().max(1);
        let mut out = image::RgbImage::new(w, h);
        let x0 = sw / 12;
        let y0 = sh / 5;
        for y in 0..h {
            for x in 0..w {
                let sx = (x0 + x) % sw;
                let sy = (y0 + y) % sh;
                out.put_pixel(x, y, *src.get_pixel(sx, sy));
            }
        }
        return out;
    }
    image::RgbImage::from_pixel(w, h, image::Rgb([0x0E, 0x0C, 0x0A]))
}

/// Gold heading as JPEG (no alpha — Adobe-safe). Flattened onto the PDF metal.
pub fn raster_gold_line(text: &str, px: f32) -> Result<(Vec<u8>, u32, u32)> {
    raster_title_line(text, px, true, false)
}

/// Cinzel title as JPEG. Gold on metal, or black on white for Formal.
pub fn raster_title_line(text: &str, px: f32, gold: bool, on_white: bool) -> Result<(Vec<u8>, u32, u32)> {
    let cinzel = FontRef::try_from_slice(CINZEL).context("Cinzel")?;
    let raj = FontRef::try_from_slice(RAJDHANI).context("Rajdhani")?;
    let scale = PxScale::from(px.max(10.0));
    let mut x = 4.0_f32;
    let mut glyphs: Vec<(FontRef, ab_glyph::Glyph)> = Vec::new();
    for ch in text.chars() {
        if ch == '\n' {
            continue;
        }
        let use_raj = cinzel.glyph_id(ch).0 == 0;
        let font = if use_raj { raj.clone() } else { cinzel.clone() };
        let scaled = font.as_scaled(scale);
        let gid = scaled.glyph_id(ch);
        let g = gid.with_scale_and_position(scale, point(x, px * 0.82));
        x += scaled.h_advance(gid);
        glyphs.push((font, g));
    }
    let w = (x + 8.0).ceil().max(8.0) as u32;
    let h = (px * 1.55).ceil().max(8.0) as u32;
    let (ink, hi) = if gold {
        ([0xD4_u8, 0xB0, 0x6A], [0xF2_u8, 0xDC, 0x9A])
    } else {
        ([0x14_u8, 0x12, 0x10], [0x4A_u8, 0x46, 0x40])
    };
    let mut img = if on_white {
        image::RgbImage::from_pixel(w, h, image::Rgb([255, 255, 255]))
    } else {
        heading_metal(w, h)
    };
    let plot = |img: &mut image::RgbImage, gx: u32, gy: u32, cover: f32, rgb: [u8; 3]| {
        if cover <= 0.02 {
            return;
        }
        if gx >= img.width() || gy >= img.height() {
            return;
        }
        let p = img.get_pixel_mut(gx, gy);
        let a = cover.clamp(0.0, 1.0);
        for i in 0..3 {
            p.0[i] = (rgb[i] as f32 * a + p.0[i] as f32 * (1.0 - a)).round() as u8;
        }
    };
    for (font, g) in &glyphs {
        if let Some(out) = font.outline_glyph(g.clone()) {
            let b = out.px_bounds();
            out.draw(|dx, dy, c| {
                plot(
                    &mut img,
                    (b.min.x + dx as f32 - 1.0).round() as u32,
                    (b.min.y + dy as f32 - 1.0).round() as u32,
                    c * 0.45,
                    hi,
                );
            });
        }
    }
    for (font, g) in &glyphs {
        if let Some(out) = font.outline_glyph(g.clone()) {
            let b = out.px_bounds();
            out.draw(|dx, dy, c| {
                plot(
                    &mut img,
                    (b.min.x + dx as f32).round() as u32,
                    (b.min.y + dy as f32).round() as u32,
                    c,
                    ink,
                );
            });
        }
    }
    let mut jpeg = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut Cursor::new(&mut jpeg), image::ImageOutputFormat::Jpeg(88))
        .map_err(|e| anyhow::anyhow!("jpeg heading: {e}"))?;
    Ok((jpeg, w, h))
}

/// Cinzel title as RGBA (transparent paper). Writers flatten onto the page JPEG.
pub fn raster_title_rgba(text: &str, px: f32, gold: bool) -> Result<(image::RgbaImage, u32, u32)> {
    let cinzel = FontRef::try_from_slice(CINZEL).context("Cinzel")?;
    let raj = FontRef::try_from_slice(RAJDHANI).context("Rajdhani")?;
    let scale = PxScale::from(px.max(10.0));
    let mut x = 4.0_f32;
    let mut glyphs: Vec<(FontRef, ab_glyph::Glyph)> = Vec::new();
    for ch in text.chars() {
        if ch == '\n' {
            continue;
        }
        let use_raj = cinzel.glyph_id(ch).0 == 0;
        let font = if use_raj { raj.clone() } else { cinzel.clone() };
        let scaled = font.as_scaled(scale);
        let gid = scaled.glyph_id(ch);
        let g = gid.with_scale_and_position(scale, point(x, px * 0.82));
        x += scaled.h_advance(gid);
        glyphs.push((font, g));
    }
    let w = (x + 8.0).ceil().max(8.0) as u32;
    let h = (px * 1.55).ceil().max(8.0) as u32;
    let ink = if gold {
        [0xD4_u8, 0xB0, 0x6A]
    } else {
        [0x14_u8, 0x12, 0x10]
    };
    let mut img = image::RgbaImage::from_pixel(w, h, image::Rgba([0, 0, 0, 0]));
    let plot = |img: &mut image::RgbaImage, gx: u32, gy: u32, cover: f32| {
        if cover <= 0.02 || gx >= img.width() || gy >= img.height() {
            return;
        }
        let a = (cover.clamp(0.0, 1.0) * 255.0).round() as u8;
        let p = img.get_pixel_mut(gx, gy);
        if a > p.0[3] {
            p.0 = [ink[0], ink[1], ink[2], a];
        }
    };
    for (font, g) in &glyphs {
        if let Some(out) = font.outline_glyph(g.clone()) {
            let b = out.px_bounds();
            out.draw(|dx, dy, c| {
                plot(
                    &mut img,
                    (b.min.x + dx as f32).round() as u32,
                    (b.min.y + dy as f32).round() as u32,
                    c,
                );
            });
        }
    }
    Ok((img, w, h))
}

/// Rajdhani body as RGBA (transparent paper). Portuguese stays in the font.
pub fn raster_body_rgba(text: &str, px: f32, gold: bool) -> Result<(image::RgbaImage, u32, u32)> {
    let cinzel = FontRef::try_from_slice(CINZEL).context("Cinzel")?;
    let raj = FontRef::try_from_slice(RAJDHANI).context("Rajdhani")?;
    let scale = PxScale::from(px.max(10.0));
    let mut x = 2.0_f32;
    let mut glyphs: Vec<(FontRef, ab_glyph::Glyph)> = Vec::new();
    for ch in text.chars() {
        if ch == '\n' {
            continue;
        }
        let use_cinzel = raj.glyph_id(ch).0 == 0;
        let font = if use_cinzel { cinzel.clone() } else { raj.clone() };
        let scaled = font.as_scaled(scale);
        let gid = scaled.glyph_id(ch);
        let g = gid.with_scale_and_position(scale, point(x, px * 0.82));
        x += scaled.h_advance(gid);
        glyphs.push((font, g));
    }
    let w = (x + 6.0).ceil().max(8.0) as u32;
    let h = (px * 1.45).ceil().max(8.0) as u32;
    let ink = if gold {
        [0xD4_u8, 0xB0, 0x6A]
    } else {
        [0xC4_u8, 0xB4, 0x90]
    };
    let mut img = image::RgbaImage::from_pixel(w, h, image::Rgba([0, 0, 0, 0]));
    let plot = |img: &mut image::RgbaImage, gx: u32, gy: u32, cover: f32| {
        if cover <= 0.02 || gx >= img.width() || gy >= img.height() {
            return;
        }
        let a = (cover.clamp(0.0, 1.0) * 255.0).round() as u8;
        let p = img.get_pixel_mut(gx, gy);
        if a > p.0[3] {
            p.0 = [ink[0], ink[1], ink[2], a];
        }
    };
    for (font, g) in &glyphs {
        if let Some(out) = font.outline_glyph(g.clone()) {
            let b = out.px_bounds();
            out.draw(|dx, dy, c| {
                plot(
                    &mut img,
                    (b.min.x + dx as f32).round() as u32,
                    (b.min.y + dy as f32).round() as u32,
                    c,
                );
            });
        }
    }
    Ok((img, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonts_load_and_have_portuguese() {
        assert!(FontRef::try_from_slice(CINZEL).is_ok());
        assert!(FontRef::try_from_slice(RAJDHANI).is_ok());
        for ch in ['ã', 'ç', 'é', 'ó', 'ê'] {
            assert!(
                font_has_char(RAJDHANI, ch),
                "Rajdhani missing {ch}"
            );
        }
        let (img, w, h) = raster_body_rgba("Relatório Identificação", 32.0, true).unwrap();
        assert!(w > 80 && h > 10, "body raster {w}x{h}");
        let ink = img.pixels().filter(|p| p.0[3] > 40).count();
        assert!(ink > 40, "Relatório must paint glyphs, not empty paper");
        let (jpeg, w, h) = raster_gold_line("MOTOR", 40.0).unwrap();
        assert!(w > 20 && h > 10);
        assert!(jpeg.starts_with(&[0xFF, 0xD8]), "jpeg soi");
        let img = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        let mixed = img.pixels().take(80).filter(|p| p.0 != [0x0E, 0x0C, 0x0A]).count();
        assert!(mixed > 8, "heading must sit on metal, not a black plaque");
        let (jpeg, w, h) = raster_title_line("RELATÓRIO", 48.0, false, true).unwrap();
        assert!(w > 20 && h > 10);
        assert!(jpeg.starts_with(&[0xFF, 0xD8]));
        let img = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        let white = img
            .pixels()
            .filter(|p| p.0[0] > 240 && p.0[1] > 240 && p.0[2] > 240)
            .count();
        assert!(white > 20, "formal title sits on white paper");
    }
}
