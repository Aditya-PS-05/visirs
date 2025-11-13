#![deny(clippy::all)]

mod visual_grouping;

use napi_derive::napi;
use napi::bindgen_prelude::*;

#[napi]
pub fn plus_100(input: u32) -> u32 {
    input + 100
}

#[napi(object)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JsAsset {
    pub id: String,
    pub name: String,
    pub path: String,
    pub mime_type: String,
    pub is_video: bool,
}


