use std::path::Path;
use anyhow::{Context, Result};
use image::DynamicImage;

/// Samples the outer ~5px ring of the image and returns the median RGB color.
/// This is used to detect the border color for bleed painting.
pub fn detect_border_color(img: &DynamicImage) -> [u8; 3] {
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    let border = 5u32;

    let mut r_vals: Vec<u8> = Vec::new();
    let mut g_vals: Vec<u8> = Vec::new();
    let mut b_vals: Vec<u8> = Vec::new();

    for y in 0..h {
        for x in 0..w {
            if x < border || x >= w.saturating_sub(border)
                || y < border || y >= h.saturating_sub(border)
            {
                let pixel = rgb.get_pixel(x, y);
                r_vals.push(pixel[0]);
                g_vals.push(pixel[1]);
                b_vals.push(pixel[2]);
            }
        }
    }

    fn median(vals: &mut Vec<u8>) -> u8 {
        if vals.is_empty() {
            return 0;
        }
        vals.sort_unstable();
        vals[vals.len() / 2]
    }

    [median(&mut r_vals), median(&mut g_vals), median(&mut b_vals)]
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
