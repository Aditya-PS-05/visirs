use anyhow::{Context, Result};
use img_hash::{HashAlg, HasherConfig, image as img_hash_image};

use std::path::Path;

/// Resize image to standard dimensions for comparison
/// Uses "Cover" to fill the entire frame, cropping the edges as needed.
/// This focuses on the central content which is most likely to be consistent
/// across different sizes and aspect ratios of the same creative
pub fn resize_for_comparison(
    img: &img_hash_image::DynamicImage,
) -> img_hash_image::ImageBuffer<img_hash_image::Rgba<u8>, Vec<u8>> {
    use img_hash_image::GenericImageView;
    let (width, height) = img.dimensions();

    let target_size = 256u32;

    let aspect_ratio = width as f64 / height as f64;
    let target_aspect = 1.0;

    let (crop_width, crop_height) = if aspect_ratio > target_aspect {
        let new_width = (height as f64 * target_aspect) as u32;
        (new_width, height)
    } else {
        let new_height = (width as f64 * target_aspect) as u32;
        (width, new_height)
    };

    let x = (width - crop_width) / 2;
    let y = (height - crop_height) / 2;

    // crop and resize
    let cropped = img.crop_imm(x, y, crop_width, crop_height);
    let resize = cropped.resize_exact(
        target_size,
        target_size,
        img_hash_image::imageops::FilterType::Lanczos3,
    );

    resize.to_rgba8()
}
