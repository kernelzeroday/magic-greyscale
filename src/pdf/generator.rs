use std::io::Cursor;
use std::path::Path;
use anyhow::{Context, Result};
use ::image::DynamicImage;
use ::image::codecs::jpeg::JpegEncoder;
use printpdf::*;
use printpdf::path::{PaintMode, WindingOrder};

use super::layout::LayoutConfig;
use crate::imaging::greyscale::detect_border_color;

fn encode_jpeg(img: &DynamicImage) -> Result<(Vec<u8>, u32, u32)> {
    let rgb = img.to_rgb8();
    let (w, h) = rgb.dimensions();
    let mut buf = Cursor::new(Vec::new());
    let encoder = JpegEncoder::new_with_quality(&mut buf, 92);
    rgb.write_with_encoder(encoder)
        .context("failed to encode JPEG")?;
    Ok((buf.into_inner(), w, h))
}

pub fn generate_pdf(
    cards: &[(DynamicImage, u32)],
    layout: &LayoutConfig,
    draw_cut_lines: bool,
    output_path: &Path,
) -> Result<()> {
    let flat_cards: Vec<&DynamicImage> = cards.iter()
        .flat_map(|(img, qty)| std::iter::repeat_n(img, *qty as usize))
        .collect();

    let page_count = layout.pages_needed(flat_cards.len());
    let pw = layout.paper.width_mm as f32;
    let ph = layout.paper.height_mm as f32;

    let (doc, page1, layer1) = PdfDocument::new(
        "Magic Greyscale Print Sheet",
        Mm(pw),
        Mm(ph),
        "Layer 1",
    );

    let mut pages: Vec<(PdfPageIndex, PdfLayerIndex)> = vec![(page1, layer1)];
    for _ in 1..page_count {
        let (page, layer) = doc.add_page(Mm(pw), Mm(ph), "Layer 1");
        pages.push((page, layer));
    }

    let positions = layout.card_positions();

    for (page_idx, chunk) in flat_cards.chunks(layout.cards_per_page()).enumerate() {
        let (page_ref, layer_ref) = &pages[page_idx];
        let layer = doc.get_page(*page_ref).get_layer(*layer_ref);

        for (slot_idx, card_img) in chunk.iter().enumerate() {
            let slot = &positions[slot_idx];

            // Paint a bleed rectangle extending 0.5mm beyond the card boundary
            // so imprecise cuts show the border color instead of white paper.
            let bleed_mm = 0.5_f32;
            let border_color = detect_border_color(card_img);
            let r_f = border_color[0] as f32 / 255.0;
            let g_f = border_color[1] as f32 / 255.0;
            let b_f = border_color[2] as f32 / 255.0;

            layer.set_fill_color(Color::Rgb(Rgb::new(r_f, g_f, b_f, None)));

            let bx1 = slot.x_mm as f32 - bleed_mm;
            let by1 = slot.y_mm as f32 - bleed_mm;
            let bx2 = slot.x_mm as f32 + layout.card_width_mm as f32 + bleed_mm;
            let by2 = slot.y_mm as f32 + layout.card_height_mm as f32 + bleed_mm;

            let bleed_rect = Polygon {
                rings: vec![vec![
                    (Point::new(Mm(bx1), Mm(by1)), false),
                    (Point::new(Mm(bx2), Mm(by1)), false),
                    (Point::new(Mm(bx2), Mm(by2)), false),
                    (Point::new(Mm(bx1), Mm(by2)), false),
                ]],
                mode: PaintMode::Fill,
                winding_order: WindingOrder::NonZero,
            };
            layer.add_polygon(bleed_rect);

            let (jpeg_data, w, h) = encode_jpeg(card_img)?;

            let image = Image::from(ImageXObject {
                width: Px(w as usize),
                height: Px(h as usize),
                color_space: ColorSpace::Rgb,
                bits_per_component: ColorBits::Bit8,
                interpolate: true,
                image_data: jpeg_data,
                image_filter: Some(ImageFilter::DCT),
                smask: None,
                clipping_bbox: None,
            });

            let dpi_x = w as f32 / (layout.card_width_mm as f32 / 25.4);
            let dpi_y = h as f32 / (layout.card_height_mm as f32 / 25.4);
            let dpi = dpi_x.min(dpi_y);

            image.add_to_layer(
                layer.clone(),
                ImageTransform {
                    translate_x: Some(Mm(slot.x_mm as f32)),
                    translate_y: Some(Mm(slot.y_mm as f32)),
                    dpi: Some(dpi),
                    ..Default::default()
                },
            );
        }

        if draw_cut_lines {
            layer.set_outline_color(Color::Greyscale(Greyscale::new(0.7, None)));
            layer.set_outline_thickness(0.5);

            for cl in &layout.cut_lines() {
                let line = Line {
                    points: vec![
                        (Point::new(Mm(cl.x1_mm as f32), Mm(cl.y1_mm as f32)), false),
                        (Point::new(Mm(cl.x2_mm as f32), Mm(cl.y2_mm as f32)), false),
                    ],
                    is_closed: false,
                };
                layer.add_line(line);
            }
        }
    }

    let pdf_bytes = doc.save_to_bytes()
        .context("failed to generate PDF")?;
    std::fs::write(output_path, pdf_bytes)
        .with_context(|| format!("failed to write PDF to {:?}", output_path))?;
    Ok(())
}
