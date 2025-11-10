use super::{Asset, AssetGroup, FrameData, HashedAsset};
use crate::visual_grouping::hash::{generate_perceptual_hash, hamming_distance};
use crate::visual_grouping::video::{
    extract_frames_from_video, get_image_dimensions, get_video_dimension,
};
use anyhow::{Context, Result};
use tempfile::TempDir;
use std::collections::HashSet;

/// Process an asset extract frame hashes
/// Returns the HashedAsset and optionally a temp directory for cleanup
pub fn process_asset(asset: &Asset) -> Result<(HashedAsset, Option<TempDir>)> {
    let (frame_paths, dimensions, temp_dir) = if asset.is_video {
        let temp_dir = TempDir::new().context("Failed to create temp directory")?;
        let frame_paths = extract_frames_from_video(&asset.path, &temp_dir)
            .context("Failed to extract frames from video")?;

        let dimensions =
            get_video_dimension(&asset.path).context("Failed to get the video dimensions")?;

        (frame_paths, dimensions, Some(temp_dir))
    } else {
        // for images, treat as a single frame
        let frame_paths = vec![asset.path.clone()];

        // Get image dimensions
        let dimensions = get_image_dimensions(&asset.path).context(
            "Failed to get image dimensions"
        )?;

        (frame_paths, dimensions, None)
    };

    let aspect_ratio = dimensions.0 as f64 / dimensions.1 as f64;

    // Generate hashes for all the images
    let mut frame_hashes = Vec::new();
    for (index, frame_path) in frame_paths.iter().enumerate() {
        let hash = generate_perceptual_hash(frame_path).context(format!("Failed to generate hash for frame {}", index))?;

        frame_hashes.push(FrameData {
           frame_number: index,
            hash,
        });
    }

    let hashed_asset = HashedAsset {
        asset: asset.clone(),
        frames: frame_hashes, 
        aspect_ratio,
        width: dimensions.0,
        height: dimensions.1,
    };

    Ok((hashed_asset, temp_dir))
}

/// Check if two assets are visually similar
/// Returns if ALL frames have hamming distance < thresold
///
/// Note: With 8-bit hashing (64-bits total), we use thresold of 15
/// which is roughly 23% of the 64-bit hash, previding good balance
pub fn are_assets_visually_similar(
    asset1: &HashedAsset, 
    asset2: &HashedAsset,
    thresold: u32,
) -> bool {
    // CRITICAL: Only campare assets of the same type (image vs video)
    // This provents videos from being grouped with images
    if asset1.asset.is_video != asset2.asset.is_video {
        return false;
    }

    // If one has significantly more frames than the other, they might still be the same video 
    // We'll compare the overlapping frame_hashes
    let min_frame_count = asset1.frames.len().min(asset2.frames.len());

    if min_frame_count == 0 {
        return false;
    }

    // Check all overlapping frames
    for i in 0..min_frame_count {
        let hash1= &asset1.frames[i].hash;
        let hash2= &asset2.frames[i].hash;

        match hamming_distance(hash1, hash2) {
            Ok(distance) => {
                if distance >= thresold {
                    return false;
                }
            }

            Err(_) => {
                return false;
            }
        }
    }

    return true;
}

/// Group assets by visual similarity
pub fn group_assets_by_visual_similarity(
    assets: Vec<Asset>,
    thresold: Option<u32>,
) -> Result<Vec<AssetGroup>> {
    let thresold = thresold.unwrap_or(15);

    if assets.is_empty() {
        return Ok(Vec::new());
    }

    println!("Processing {} assets for visual grouping...", assets.len());

    // Process all assets to extract frames and generate hashes
    // keep temp directories alive until grouping is complete 
    let process_results: Vec<(HashedAsset, Option<TempDir>)> = assets.iter().map(|asset| {
        println!(
            "Processing asset: {} ({})", 
            asset.name,
            if asset.is_video {"video"} else {"image"}
        );
        let result = process_asset(asset)?;
        println!("Completed processing: {}", asset.name);
        Ok(result)
    }).collect::<Result<Vec<_>>>()?;

    let hashed_assets: Vec<HashedAsset> = process_results.iter().map(|(hashed_asset, _)| hashed_asset.clone()).collect();

    println!("Generated hashes for {} assets", hashed_assets.len());

    // Group assets by visual similarity
    let mut groups: Vec<AssetGroup> = Vec::new();
    let mut assigned: HashSet<String> = HashSet::new();

    for i in 0..hashed_assets.len() {
        if assigned.contains(&hashed_assets[i].asset.id) {
            continue;
        }

        let mut group = AssetGroup {
            id: uuid::Uuid::new_v4().to_string(),
            name: extract_base_name(&hashed_assets[i].asset.name),
            assets: vec![hashed_assets[i].asset.clone()],
        };

        assigned.insert(hashed_assets[i].asset.id.clone());

        // Find all similar assets 
        for j in (i+1)..hashed_assets.len() {
            if assigned.contains(&hashed_assets[j].asset.id) {
                continue;
            }

            let is_similar = are_assets_visually_similar(&hashed_assets[i], &hashed_assets[j], thresold);

            // Debug logging
            if !hashed_assets[i].frames.is_empty() && !hashed_assets[j].frames.is_empty() {
                if let Ok(distance) = hamming_distance(
                    &hashed_assets[i].frames[0].hash,
                    &hashed_assets[j].frames[0].hash,
                ) {
                    let type1 = if hashed_assets[i].asset.is_video {"video"} else {"image"};
                    let type2 = if hashed_assets[j].asset.is_video {"video"} else {"image"};
                    println!(
                        "Comparing {} \"{}]\" vs {} \"{}\": distance={}, similar={}",
                        type1, hashed_assets[i].asset.name,
                        type2, hashed_assets[j].asset.name,
                        distance, is_similar
                    );
                }
            }

            if is_similar {
                group.assets.push(hashed_assets[j].asset.clone());
                assigned.insert(hashed_assets[j].asset.id.clone());
            }
        }

        groups.push(group);
    }

    println!(
        "Created {} visual groups from {} assets",
        groups.len(),
        assets.len()
    );

    Ok(groups)
}

/// Extract base name from filename (remove extension and common suffixes) 
fn extract_base_name(filename: &str) -> String {
    let base = filename.rsplit_once('.')
        .map(|(name, _)| name)
        .unwrap_or(filename);

    let base = regex::Regex::new(r"[_\-\s]*((\d{1,4})\s*[:xXw×]\s*(\d{1,4}))$")
        .ok()
        .and_then(|re| re.replace(base, "").into_owned().into())
        .unwrap_or_else (|| base.to_string());

    let base = regex::Regex::new(r"(?i)[_\-\s]*(post|story|feed|infeed|square|vertical|horizontal)$")
        .ok()
        .and_then(|re| re.replace(&base, "").into_owned().into())
        .unwrap_or(base);

    base.trim().to_string()
}
