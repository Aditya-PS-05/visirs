pub mod grouping;
pub mod hash;
pub mod video;

use serde::{Deserialize, Serialize};

/// Asset type with file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub path: String,
    pub mime_type: String,
    pub is_video: bool,
}

/// Frame data with hash
#[derive(Debug, Clone)]
pub struct FrameData {
    pub frame_number: usize,
    pub hash: Vec<u8>,
}

/// Asset with extracted frame hashes
#[derive(Debug, Clone)]
pub struct HashedAsset {
    pub asset: Asset,
    pub frames: Vec<FrameData>,
    pub aspect_ratio: f64,
    pub width: u32,
    pub height: u32,
}

/// Group of visually similar assets
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetGroup {
    pub id: String,
    pub name: String,
    pub assets: Vec<Asset>,
}
