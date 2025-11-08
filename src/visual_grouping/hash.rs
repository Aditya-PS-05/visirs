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

pub fn generate_perceptual_hash<P: AsRef<Path>>(image_path: P) -> Result<Vec<u8>> {
    let img = img_hash_image::open(image_path.as_ref()).context("Failed to open image")?;

    let resized = resize_for_comparison(&img);

    let dynamic_img = img_hash_image::DynamicImage::ImageRgba8(resized);

    let hasher = HasherConfig::new()
        .hash_alg(HashAlg::Blockhash)
        .hash_size(8, 8)
        .to_hasher();

    let hash = hasher.hash_image(&dynamic_img);

    Ok(hash.as_bytes().to_vec())
}

pub fn hamming_distance(hash1: &[u8], hash2: &[u8]) -> Result<u32> {
    if hash1.len() != hash2.len() {
        anyhow::bail!("Hashes must be the same length");
    }
    let mut distance = 0u32;
    for (byte1, byte2) in hash1.iter().zip(hash2.iter()) {
        let xor = byte1 ^ byte2;

        distance += xor.count_ones();
    }

    Ok(distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hamming_distance() {
        let hash1 = vec![0b11110000, 0b10101010];
        let hash2 = vec![0b11110000, 0b10101010];
        assert_eq!(hamming_distance(&hash1, &hash2).unwrap(), 0);

        let hash3 = vec![0b11110000, 0b00000000];
        let hash4 = vec![0b00001111, 0b11111111];
        assert_eq!(hamming_distance(&hash3, &hash4).unwrap(), 16);
    }
}
