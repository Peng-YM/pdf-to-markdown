use pdfium_render::prelude::{PdfBitmap, PdfDocument, PdfPage, PdfPageIndex, Pdfium, Pixels};
use std::path::Path;

const DISABLE_ENV_VAR: &str = "PDF_TO_MARKDOWN_NO_PDF_CROP";

/// Check if PDF cropping is disabled via environment variable.
pub fn is_pdf_crop_disabled() -> bool {
    std::env::var(DISABLE_ENV_VAR).map(|v| v == "1" || v.to_lowercase() == "true").unwrap_or(false)
}

/// Parse bounding box coordinates from a PaddleOCR image path.
/// Image paths are like: `imgs/img_in_chart_box_216_581_964_894.jpg`
/// Returns (x1, y1, x2, y2) if parsing succeeds.
pub fn parse_bbox_from_image_path(path: &str) -> Option<(f64, f64, f64, f64)> {
    let box_marker = "_box_";
    let box_pos = path.rfind(box_marker)?;
    let after_box = &path[box_pos + box_marker.len()..];

    let parts: Vec<&str> = after_box.split('_').collect();
    if parts.len() < 4 {
        return None;
    }

    let x1: f64 = parts[0].parse().ok()?;
    let y1: f64 = parts[1].parse().ok()?;
    let x2: f64 = parts[2].parse().ok()?;
    // Last part may include file extension, strip it
    let y2_str = parts[3].rsplit('.').nth(1).unwrap_or(parts[3]);
    let y2: f64 = y2_str.parse().ok()?;

    Some((x1, y1, x2, y2))
}

/// Render a PDF page and crop a bounding box region, returning high-res JPEG bytes.
///
/// `page_idx` is 0-indexed. `bbox` is in PaddleOCR coordinate space
/// defined by `ref_width` × `ref_height` (from the JSONL prunedResult).
/// The page is rendered at a configurable DPI (env `PDF_TO_MARKDOWN_PDF_CROP_DPI`,
/// default 300) and the bbox is scaled proportionally.
pub fn extract_high_res_image(
    pdf_path: &Path,
    page_idx: u32,
    bbox: (f64, f64, f64, f64),
    ref_width: f64,
    ref_height: f64,
) -> Option<Vec<u8>> {
    let (x1, y1, x2, y2) = bbox;

    let target_dpi: f64 = std::env::var("PDF_TO_MARKDOWN_PDF_CROP_DPI")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300.0);

    let pdfium: Pdfium = pdfium_auto::bind_pdfium_silent().ok()?;
    let doc: PdfDocument = pdfium.load_pdf_from_file(pdf_path, None).ok()?;
    let page: PdfPage = doc.pages().get(page_idx as PdfPageIndex).ok()?;

    // Read actual page dimensions in points and compute render size at target DPI
    let page_w_pts = page.width().value as f64;
    let page_h_pts = page.height().value as f64;
    let target_width = (page_w_pts / 72.0 * target_dpi).round() as Pixels;
    let target_height = (page_h_pts / 72.0 * target_dpi).round() as Pixels;

    let bitmap: PdfBitmap = page.render(target_width, target_height, None).ok()?;

    let rendered_width = bitmap.width() as f64;
    let rendered_height = bitmap.height() as f64;

    // Scale bbox from PaddleOCR coordinate space to rendered pixel space
    let scale_x = rendered_width / ref_width;
    let scale_y = rendered_height / ref_height;

    let rx1 = (x1 * scale_x).round().max(0.0) as u32;
    let ry1 = (y1 * scale_y).round().max(0.0) as u32;
    let rx2 = (x2 * scale_x).round().min(rendered_width) as u32;
    let ry2 = (y2 * scale_y).round().min(rendered_height) as u32;

    let crop_w = rx2.saturating_sub(rx1).max(1);
    let crop_h = ry2.saturating_sub(ry1).max(1);

    let rgba_bytes = bitmap.as_rgba_bytes();

    let img =
        image::RgbaImage::from_raw(bitmap.width() as u32, bitmap.height() as u32, rgba_bytes)?;

    let cropped = image::DynamicImage::ImageRgba8(img).crop_imm(rx1, ry1, crop_w, crop_h);

    let mut buf = Vec::new();
    cropped.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Jpeg).ok()?;

    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bbox() {
        let cases = [
            ("imgs/img_in_chart_box_216_581_964_894.jpg", Some((216.0, 581.0, 964.0, 894.0))),
            ("imgs/img_in_image_box_220_879_1003_1237.jpg", Some((220.0, 879.0, 1003.0, 1237.0))),
            // After / → _ replacement and safe_img_name transform
            ("imgs_img_in_image_box_216_158_1008_795.jpg", Some((216.0, 158.0, 1008.0, 795.0))),
            ("no_box_here.jpg", None),
            ("img_box_1_2.jpg", None),
        ];

        for (input, expected) in cases {
            assert_eq!(parse_bbox_from_image_path(input), expected);
        }
    }
}
