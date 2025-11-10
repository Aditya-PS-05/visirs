use anyhow::{Context, Result};
use ffmpeg_next as ffmpeg;
use image::{save_buffer, GenericImageView};
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

    let mut input = ffmpeg::format::input(&video_path)
        .context("Failed to open video file for frame extraction")?;

    let video_stream_index = input
        .streams()
        .best(ffmpeg::media::Type::Video)
        .context("Could  not find video stream")?
        .index();

    let video_stream = input
        .stream(video_stream_index)
        .context("Failed to get Video stream")?;

    let context_decoder =
        ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
            .context("Failed to create codec context")?;

    let mut decoder = context_decoder
        .decoder()
        .video()
        .context("Failed to create video decoder")?;

    let mut scaler = ffmpeg::software::scaling::context::Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg::format::Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        ffmpeg::software::scaling::flag::Flags::BILINEAR,
    )
    .context("Failed to create scaler")?;

    let mut frame_paths: Vec<String> = Vec::new();
    let mut decoded_frame = ffmpeg::util::frame::video::Video::empty();
    let time_base = input.stream(video_stream_index).unwrap().time_base();

    // Seek and decode frames

    for (idx, target_time) in frame_times.iter().enumerate() {
        let timestamp = (target_time / f64::from(time_base)) as i64;
        input
            .seek(timestamp, ..timestamp)
            .context(format!("Failed to seek to time {:.2}s", target_time))?;

        let mut found_frame = false;
        for (stream, packet) in input.packets() {
            if stream.index() == video_stream_index {
                decoder.send_packet(&packet).ok();

                while decoder.receive_frame(&mut decoded_frame).is_ok() {
                    let pts = decoded_frame.pts().unwrap_or(0);
                    let current_time = pts as f64 * f64::from(time_base);
                    if (current_time - target_time).abs() < frame_interval / 2.0 {
                        // Convert frame to RGB24
                        let mut rgb_frame = ffmpeg::util::frame::video::Video::empty();
                        scaler
                            .run(&decoded_frame, &mut rgb_frame)
                            .context("Failed to scale frame")?;

                        let frame_path = temp_dir.path().join(format!("frame_{}.png", idx));

                        save_frame_as_png(&rgb_frame, &frame_path)
                            .context(format!("Failed to save frame {}", idx))?;

                        println!(
                            "Extracted frame {} at {:.2}s -> {:?}",
                            idx, current_time, frame_path
                        );

                        frame_paths.push(frame_path.to_string_lossy().to_string());
                        found_frame = true;
                    }
                }

                if found_frame {
                    break;
                }
            }
        }
    }

    decoder.send_eof().ok();
    while decoder.receive_frame(&mut decoded_frame).is_ok() {
        // process any remaining frames if needed
    }

    if frame_paths.is_empty() {
        anyhow::bail!("Failed to extract any frames from video");
    }

    println!("Successfully extracted {} frames", frame_paths.len());

    Ok(frame_paths)
}

/// Save a video frame as PNG
fn save_frame_as_png<P: AsRef<Path>>(
    frame: &ffmpeg::util::frame::video::Video,
    output_path: P,
) -> Result<()> {
    let width = frame.width();
    let height = frame.height();

    let data = frame.data(0);
    let img_buffer = image::RgbImage::from_raw(width, height, data.to_vec())
        .context("Failed tp create image buffer from frame")?;

    img_buffer
        .save(output_path.as_ref())
        .context("Failed to save frame as PNG")?;

    Ok(())
}

// Get Image Dimensions
pub fn get_image_dimensions<P: AsRef<Path>>(image_path: P) -> Result<(u32, u32)> {
    let img = image::open(image_path.as_ref()).context("Failed to open image")?;
    Ok(img.dimensions())
}
