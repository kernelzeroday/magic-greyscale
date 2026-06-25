use std::path::Path;
use anyhow::{Context, Result};
use image::DynamicImage;

/// Fills the white rounded-corner areas of a card image with the detected
/// border color so that adjacent cards on a print sheet have no white seams.
pub fn fill_rounded_corners(img: &mut DynamicImage) {
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();

    // Sample the edge midpoints to find the border color (avoiding corners)
    let mut samples = Vec::new();
    let mid_x = w / 2;
    let mid_y = h / 2;
    for &x in &[mid_x.saturating_sub(20), mid_x, mid_x.saturating_add(20).min(w - 1)] {
        samples.push(rgb.get_pixel(x, 0));
        samples.push(rgb.get_pixel(x, h - 1));
    }
    for &y in &[mid_y.saturating_sub(20), mid_y, mid_y.saturating_add(20).min(h - 1)] {
        samples.push(rgb.get_pixel(0, y));
        samples.push(rgb.get_pixel(w - 1, y));
    }

    fn median_channel(vals: &mut Vec<u8>) -> u8 {
        vals.sort_unstable();
        vals[vals.len() / 2]
    }

    let border_r = median_channel(&mut samples.iter().map(|p| p[0]).collect());
    let border_g = median_channel(&mut samples.iter().map(|p| p[1]).collect());
    let border_b = median_channel(&mut samples.iter().map(|p| p[2]).collect());

    let border_pixel = image::Rgb([border_r, border_g, border_b]);

    // The rounded corner radius on card images is roughly 3% of the shorter dimension
    let corner_radius = (w.min(h) as f64 * 0.04) as u32;
    let threshold = 180u8;

    let mut out = rgb.clone();

    for y in 0..h {
        for x in 0..w {
            let in_corner = (x < corner_radius && y < corner_radius)
                || (x >= w - corner_radius && y < corner_radius)
                || (x < corner_radius && y >= h - corner_radius)
                || (x >= w - corner_radius && y >= h - corner_radius);

            if !in_corner {
                continue;
            }

            let p = rgb.get_pixel(x, y);
            let brightness = (p[0] as u16 + p[1] as u16 + p[2] as u16) / 3;
            if brightness as u8 > threshold {
                out.put_pixel(x, y, border_pixel);
            }
        }
    }

    *img = DynamicImage::ImageRgb8(out);
}

pub fn convert_to_greyscale(
    image_path: &Path,
    contrast: f32,
    brightness: i32,
    toner_save: u32,
) -> Result<DynamicImage> {
    let img = image::open(image_path)
        .with_context(|| format!("failed to open image: {:?}", image_path))?;
    let grey = img.grayscale();
    let adjusted = grey.adjust_contrast(contrast);
    let mut final_img = adjusted.brighten(brightness);
    if toner_save > 0 {
        apply_toner_save(&mut final_img, toner_save);
    }
    Ok(final_img)
}

fn apply_toner_save(img: &mut DynamicImage, amount: u32) {
    let factor = amount.min(100) as f32 / 100.0;
    let floor = (factor * 180.0) as u8;
    let rgb = img.as_mut_rgb8().or_else(|| None);
    if let Some(rgb) = rgb {
        for pixel in rgb.pixels_mut() {
            for c in pixel.0.iter_mut() {
                if *c < floor {
                    *c = floor + ((*c as f32 / floor as f32) * (255 - floor) as f32) as u8;
                }
            }
        }
        return;
    }
    if let Some(luma) = img.as_mut_luma8() {
        for pixel in luma.pixels_mut() {
            if pixel.0[0] < floor {
                pixel.0[0] = floor + ((pixel.0[0] as f32 / floor as f32) * (255 - floor) as f32) as u8;
            }
        }
    }
}
