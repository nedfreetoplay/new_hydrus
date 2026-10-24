//! Integration tests against a real FFmpeg executable.
//!
//! These tests are ignored by default because they need `ffmpeg` (and a build
//! with libx264/libmp3lame) on `PATH`. Run them with:
//!
//! ```text
//! cargo test --test ffmpeg_integration -- --ignored --nocapture
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use hydrus_core::files::hydrus_ffmpeg;
use hydrus_core::files::hydrus_ffmpeg_rendering;
use hydrus_core::files::hydrus_file_handling;
use hydrus_core::files::hydrus_video_handling::{self, VideoRendererFFMPEG};
use hydrus_core::hydrus_constants::Mime;

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the clock is after the epoch")
            .as_nanos();

        let path = std::env::temp_dir().join(format!(
            "hydrus_ffmpeg_it_{name}_{}_{}",
            std::process::id(),
            unique
        ));

        std::fs::create_dir_all(&path).expect("the test dir can be created");

        Self { path }
    }

    fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn run_ffmpeg(args: &[&str]) {
    let output = Command::new(hydrus_ffmpeg::get_current_ffmpeg_path())
        .args(["-hide_banner", "-loglevel", "error", "-y", "-nostdin"])
        .args(args)
        .output()
        .expect("ffmpeg can be executed");

    assert!(
        output.status.success(),
        "ffmpeg failed with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn make_video(path: &Path, with_audio: bool) {
    let path = path.to_string_lossy().into_owned();

    if with_audio {
        run_ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=176x144:rate=10",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100",
            "-t",
            "1.0",
            "-pix_fmt",
            "yuv420p",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-c:a",
            "aac",
            "-b:a",
            "64k",
            "-shortest",
            &path,
        ]);
    } else {
        run_ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x240:rate=10",
            "-t",
            "2.0",
            "-pix_fmt",
            "yuv420p",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-movflags",
            "+faststart",
            &path,
        ]);
    }
}

fn make_mp3(path: &Path) {
    run_ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:sample_rate=44100",
        "-t",
        "1.0",
        "-c:a",
        "libmp3lame",
        "-b:a",
        "64k",
        &path.to_string_lossy(),
    ]);
}

fn make_mp3_with_cover(path: &Path, cover_path: &Path) {
    run_ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "color=c=red:s=64x64",
        "-frames:v",
        "1",
        &cover_path.to_string_lossy(),
    ]);

    run_ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=440:sample_rate=44100",
        "-i",
        &cover_path.to_string_lossy(),
        "-t",
        "1.0",
        "-map",
        "0:a",
        "-map",
        "1:v",
        "-c:a",
        "libmp3lame",
        "-b:a",
        "64k",
        "-c:v",
        "png",
        "-disposition:v",
        "attached_pic",
        "-id3v2_version",
        "3",
        "-metadata:s:v",
        "title=Album cover",
        "-metadata:s:v",
        "comment=Cover (front)",
        &path.to_string_lossy(),
    ]);
}

#[test]
#[ignore = "requires ffmpeg on PATH"]
fn ffmpeg_reports_a_version() {
    let version = hydrus_ffmpeg::getffmpeg_version();

    assert!(
        !version.starts_with("no ffmpeg")
            && !version.starts_with("unable to execute")
            && version != "unknown",
        "hydrus could not run ffmpeg: {version}"
    );

    let first_component = version.split('.').next().unwrap_or_default();

    assert!(
        !first_component.is_empty() && first_component.chars().all(|c| c.is_ascii_digit()),
        "unexpected ffmpeg version: {version}"
    );

    println!("ffmpeg version via hydrus_core: {version}");
}

#[test]
#[ignore = "requires ffmpeg on PATH"]
fn probes_video_properties_and_renders_frames() {
    let dir = TestDir::new("video");
    let path = dir.join("sample.mp4");

    make_video(&path, false);

    let (resolution, duration_ms, num_frames, has_audio) =
        hydrus_video_handling::get_ffmpeg_video_properties(&path, false)
            .expect("the synthetic MP4 is readable");

    assert_eq!(resolution, (320, 240));
    assert!(
        (1_800..=2_300).contains(&duration_ms),
        "unexpected duration: {duration_ms} ms"
    );
    assert!(
        (18..=22).contains(&num_frames),
        "unexpected frame count: {num_frames}"
    );
    assert!(!has_audio);

    let png = hydrus_ffmpeg_rendering::render_image_to_png_bytes(&path)
        .expect("the first frame can be rendered as PNG");

    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));

    let image = hydrus_core::files::images::hydrus_image_handling::decode_image_bytes(&png)
        .expect("the rendered PNG decodes");
    let rgb = image.to_rgb8();

    assert_eq!((rgb.width(), rgb.height()), (320, 240));

    let mut renderer = VideoRendererFFMPEG::new(
        &path,
        Mime::Mp4Video,
        Some(duration_ms),
        num_frames,
        (160, 120),
        None,
        None,
    )
    .expect("the streaming renderer starts");

    let first_frame = renderer.read_frame().expect("the first frame streams");

    assert_eq!((first_frame.width, first_frame.height), (160, 120));
    assert_eq!(first_frame.depth, 3);
    assert_eq!(first_frame.data.len(), 160 * 120 * 3);
    assert_eq!(
        first_frame
            .to_dynamic_image()
            .expect("the raw frame converts")
            .to_rgb8()
            .dimensions(),
        (160, 120)
    );

    renderer
        .set_position(10)
        .expect("the renderer can seek forward");
    let later_frame = renderer
        .read_frame()
        .expect("a frame after seeking streams");

    assert_eq!((later_frame.width, later_frame.height), (160, 120));
    assert_eq!(later_frame.data.len(), 160 * 120 * 3);

    renderer.stop();

    let thumbnail = hydrus_file_handling::generate_thumbnail_numpy(
        &path,
        (64, 64),
        Mime::Mp4Video,
        Some(duration_ms),
        Some(num_frames),
        35.0,
    )
    .expect("the video thumbnail renders");
    let thumbnail = thumbnail.to_rgb8();

    assert_eq!((thumbnail.width(), thumbnail.height()), (64, 64));
}

#[test]
#[ignore = "requires ffmpeg on PATH"]
fn detects_video_with_audio_and_probes_file_info() {
    let dir = TestDir::new("audio_video");
    let path = dir.join("sample_with_audio.mp4");

    make_video(&path, true);

    let mime =
        hydrus_file_handling::get_mime(&path, false).expect("the MP4 MIME type is recognised");
    assert_eq!(mime, Mime::Mp4Video);

    let info = hydrus_file_handling::get_file_info(&path, None, false)
        .expect("the MP4 file info is collected");

    assert_eq!(info.mime, Mime::Mp4Video);
    assert_eq!((info.width, info.height), (Some(176), Some(144)));
    assert!(
        (800..=1_300).contains(&info.duration_ms.unwrap_or_default()),
        "unexpected duration: {:?}",
        info.duration_ms
    );
    assert!(
        (8..=12).contains(&info.num_frames.unwrap_or_default()),
        "unexpected frame count: {:?}",
        info.num_frames
    );
    assert!(info.has_audio);
}

#[test]
#[ignore = "requires ffmpeg on PATH"]
fn detects_mp3_and_extracts_attached_cover() {
    let dir = TestDir::new("audio_cover");
    let plain_path = dir.join("plain.mp3");
    let cover_path = dir.join("cover.png");
    let covered_path = dir.join("covered.mp3");

    make_mp3(&plain_path);

    let info = hydrus_file_handling::get_file_info(&plain_path, None, false)
        .expect("the MP3 file info is collected");

    assert_eq!(info.mime, Mime::Mp3);
    assert_eq!(info.width, None);
    assert_eq!(info.height, None);
    assert!(info.has_audio);
    assert!(
        (800..=1_300).contains(&info.duration_ms.unwrap_or_default()),
        "unexpected duration: {:?}",
        info.duration_ms
    );

    make_mp3_with_cover(&covered_path, &cover_path);

    let rendered_path = dir.join("cover-rendered.png");

    assert!(
        hydrus_ffmpeg_rendering::render_any_attached_still_image_to_path(
            &covered_path,
            &rendered_path
        )
        .expect("the attached cover is requested"),
        "FFmpeg did not expose an attached cover stream"
    );

    let cover = std::fs::read(&rendered_path).expect("the rendered cover exists");
    let image = hydrus_core::files::images::hydrus_image_handling::decode_image_bytes(&cover)
        .expect("the rendered cover decodes");
    let rgb = image.to_rgb8();

    assert_eq!((rgb.width(), rgb.height()), (64, 64));
}
