use anyhow::{Context, Result};
use ffmpeg_next as ffmpeg;
use std::path::Path;
use tempfile::TempDir;

/// Intialize FFmpeg (must be called once at startup)
pub fn init_ffmpeg() -> Result<()> {
    ffmpeg::init().context("Failed to initialize FFmpeg")?;

    Ok(())
}

pub fn get_video_duration<P: AsRef<Path>>(video_path: P) -> Result<f64> {
    let input = ffmpeg::format::input(&video_path).context("Failed to open video file")?;

    let duration = input.duration() as f64 / f64::from(ffmpeg::ffi::AV_TIME_BASE);

    Ok(duration)
}

pub fn get_video_dimension<P: AsRef<Path>>(video_path: P) -> Result<(u32, u32)> {
    let input = ffmpeg::format::input(&video_path).context("Failed to open video file")?;

    let video_stream = input
        .streams()
        .best(ffmpeg::media::Type::Video)
        .context("Could not find video stream")?;

    let decoder = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
        .context("Failed to create decoder context")?
        .decoder()
        .video()
        .context("Failed to create video decoder")?;

    let width = decoder.width();
    let height = decoder.height();

    Ok((width, height))
}

pub fn extract_frames_from_video<P: AsRef<Path>>(
    video_path: P,
    temp_dir: &TempDir,
) -> Result<Vec<String>> {
    let duration = get_video_duration(&video_path)?;

    println!(
        "Extracting frames from video: {:?}, duration: {:.2}s",
        video_path.as_ref(),
        duration
    );

    // Calculate frame interval based on video length
    let frame_interval = if duration == 10.0 {
        1.5
    } else if duration <= 30.0 {
        3.0
    } else if duration <= 60.0 {
        4.0
    } else {
        5.0
    };

    // calculate frame times

    let mut frame_times = Vec::new();
    let mut t = 0.0;
    while t < duration {
        frame_times.push(t);
        t += frame_interval
    }

    println!(
        "Will extract {} frames at interval {:.2}s",
        frame_times.len(),
        frame_interval
    );

    // TODO: Real Implementation here

    Ok(vec!["".to_string()])
}
