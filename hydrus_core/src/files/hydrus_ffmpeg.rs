//! Rust port of `hydrus/core/files/HydrusFFMPEG.py`.
//!
//! Hydrus invokes the `ffmpeg` executable rather than linking a codec library.
//! The port keeps that design: [`hydrus_subprocess`] drains stderr, and the
//! small parsers in [`crate::files::hydrus_ffmpeg_parsing`] read the stream lines.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::hydrus_constants::{PLATFORM_WINDOWS, dirs};
use crate::hydrus_data;
use crate::hydrus_environment;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::processes::hydrus_subprocess::{self, SubprocessCommand, SubprocessResult};

/// `FFMPEG_SUBPROCESS_TIMEOUT`
pub const FFMPEG_SUBPROCESS_TIMEOUT: u64 = 15;

/// `FFMPEG_EXE_NAME`
pub const FFMPEG_EXE_NAME: &str = if PLATFORM_WINDOWS {
    "ffmpeg.exe"
} else {
    "ffmpeg"
};

static FFMPEG_MISSING_ERROR_PUBBED: AtomicBool = AtomicBool::new(false);
static FFMPEG_NO_CONTENT_ERROR_PUBBED: AtomicBool = AtomicBool::new(false);
static HYDRUS_BIN_FFMPEG_WAS_LOOKED_FOR: AtomicBool = AtomicBool::new(false);
static HYDRUS_BIN_FFMPEG_EXISTS: AtomicBool = AtomicBool::new(false);
static PREFER_SYSTEM_FFMPEG: AtomicBool = AtomicBool::new(false);

/// `PREFER_SYSTEM_FFMPEG`
pub fn prefer_system_ffmpeg() -> bool {
    PREFER_SYSTEM_FFMPEG.load(Ordering::Relaxed)
}

/// Set `PREFER_SYSTEM_FFMPEG`.
pub fn set_prefer_system_ffmpeg(prefer_system: bool) {
    PREFER_SYSTEM_FFMPEG.store(prefer_system, Ordering::Relaxed);
}

/// `get_current_ffmpeg_path()`
pub fn get_current_ffmpeg_path() -> String {
    if !prefer_system_ffmpeg() {
        let bundled_path = dirs().bin_dir.join(FFMPEG_EXE_NAME);

        if !HYDRUS_BIN_FFMPEG_WAS_LOOKED_FOR.swap(true, Ordering::Relaxed) {
            HYDRUS_BIN_FFMPEG_EXISTS.store(bundled_path.exists(), Ordering::Relaxed);
        }

        if HYDRUS_BIN_FFMPEG_EXISTS.load(Ordering::Relaxed) {
            return bundled_path.to_string_lossy().into_owned();
        }
    }

    FFMPEG_EXE_NAME.to_string()
}

/// `check_ffmpeg_error( lines )`
pub fn check_ffmpeg_error(lines: &[String]) -> HydrusResult<()> {
    let Some(last_line) = lines.last() else {
        return Err(HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "Could not parse that file--no FFMPEG output given.",
        ));
    };

    if last_line.contains("No such file or directory") {
        return Err(HydrusException::new(ErrorKind::FileMissing, "File not found!"));
    }

    if last_line.contains("Invalid data") {
        return Err(HydrusException::new(
            ErrorKind::DamagedOrUnusualFile,
            "FFMPEG could not parse.",
        ));
    }

    Ok(())
}

/// The `ffmpeg -i` command, with an optional manual frame count pass.
pub fn build_info_command(
    path: &str,
    video_stream_mapping_to_count_frames_manually: Option<&str>,
) -> Vec<String> {
    let mut cmd = vec![
        get_current_ffmpeg_path(),
        "-xerror".to_string(),
        "-i".to_string(),
        path.to_string(),
    ];

    if let Some(mapping) = video_stream_mapping_to_count_frames_manually {
        cmd.extend([
            "-map".to_string(),
            mapping.to_string(),
            "-vf".to_string(),
            "scale=-2:120".to_string(),
            "-an".to_string(),
            "-f".to_string(),
            "null".to_string(),
            if PLATFORM_WINDOWS {
                "NUL".to_string()
            } else {
                "/dev/null".to_string()
            },
        ]);
    }

    cmd
}

/// Run an FFMPEG command, translating the two errors callers expect.
pub(crate) fn run_ffmpeg_command(
    args: &[String],
    text: bool,
    bufsize: usize,
    path_for_error: &Path,
    timeout_message: &str,
) -> HydrusResult<SubprocessResult> {
    let command = SubprocessCommand::args(args.iter().cloned());

    match hydrus_subprocess::run_subprocess(
        &command,
        FFMPEG_SUBPROCESS_TIMEOUT,
        bufsize,
        false,
        true,
        text,
        false,
    ) {
        Ok(result) => Ok(result),
        Err(error) if error.is_a(ErrorKind::SubprocessTimedOut) => Err(
            HydrusException::new(ErrorKind::DamagedOrUnusualFile, timeout_message),
        ),
        Err(error) if error.is_a(ErrorKind::Executable) => {
            Err(handle_ffmpeg_file_not_found_and_generate_exception(
                &error,
                path_for_error,
            ))
        }
        Err(error) => Err(error),
    }
}

/// `get_ffmpeg_info_lines( path, video_stream_mapping_to_count_frames_manually )`
pub fn get_ffmpeg_info_lines(
    path: impl AsRef<Path>,
    video_stream_mapping_to_count_frames_manually: Option<&str>,
) -> HydrusResult<Vec<String>> {
    let path = path.as_ref();

    hydrus_data::check_program_is_not_shutting_down()?;

    let cmd = build_info_command(
        &path.to_string_lossy(),
        video_stream_mapping_to_count_frames_manually,
    );

    let result = run_ffmpeg_command(
        &cmd,
        true,
        65_536,
        path,
        "ffmpeg could not read file info quick enough!",
    )?;

    let stderr = result.stderr_text();

    if stderr.is_empty() {
        return Err(handle_ffmpeg_no_content_and_generate_exception(
            path,
            &result.stdout_text(),
            &stderr,
        ));
    }

    let lines: Vec<String> = stderr.lines().map(|line| line.trim().to_string()).collect();

    check_ffmpeg_error(&lines)?;

    Ok(lines)
}

/// `getffmpeg_version()`
pub fn getffmpeg_version() -> String {
    let ffmpeg_path = get_current_ffmpeg_path();

    if hydrus_data::check_program_is_not_shutting_down().is_err() {
        return format!("unable to execute ffmpeg at path \"{ffmpeg_path}\"");
    }

    let command = SubprocessCommand::args([ffmpeg_path.clone(), "-version".to_string()]);

    let result = match hydrus_subprocess::run_subprocess(
        &command,
        FFMPEG_SUBPROCESS_TIMEOUT,
        65_536,
        false,
        true,
        true,
        false,
    ) {
        Ok(result) => result,
        Err(error) if error.is_a(ErrorKind::Executable) => {
            return format!("no ffmpeg found at path \"{ffmpeg_path}\"");
        }
        Err(error) if error.is_a(ErrorKind::SubprocessTimedOut) => {
            return format!("ffmpeg took too long to respond from path \"{ffmpeg_path}\"");
        }
        Err(error) => {
            hydrus_data::safe_print(error.to_string());

            return format!("unable to execute ffmpeg at path \"{ffmpeg_path}\"");
        }
    };

    let stdout = result.stdout_text();
    let stderr = result.stderr_text();

    if let Some(top_line) = stdout.lines().next() {
        if let Some(rest) = top_line.strip_prefix("ffmpeg version ") {
            if let Some(version_string) = rest.split_whitespace().next() {
                return version_string.to_string();
            }
        }
    }

    let mut message = "FFMPEG was recently contacted to fetch version information. While FFMPEG could be found, the response could not be understood. Significant debug information has been printed to the log, which hydrus_dev would be interested in.".to_string();

    hydrus_data::show_text(&message);

    message.push_str("\n\n");
    message.push_str(&format!("STDOUT Response: {stdout}"));
    message.push_str("\n\n");
    message.push_str(&format!("STDERR Response: {stderr}"));

    hydrus_data::safe_print(&message);

    hydrus_environment::dump_env(None);

    FFMPEG_NO_CONTENT_ERROR_PUBBED.store(true, Ordering::Relaxed);

    "unknown".to_string()
}

/// `handle_ffmpeg_file_not_found_and_generate_exception( e, path )`
pub fn handle_ffmpeg_file_not_found_and_generate_exception(
    error: &HydrusException,
    path: impl AsRef<Path>,
) -> HydrusException {
    if !FFMPEG_MISSING_ERROR_PUBBED.swap(true, Ordering::Relaxed) {
        let mut message = format!(
            "FFMPEG, which hydrus uses to parse and render some media, and here was trying to look at \"{}\", was not found! This may be due to it not being available on your system--or hydrus just being unable to find it.",
            path.as_ref().display()
        );

        message.push_str("\n\n");

        if PLATFORM_WINDOWS {
            message.push_str("You are on Windows, so there should be a copy of ffmpeg.exe in your install_dir/bin folder. If not, please check if your anti-virus has removed it and restore it through a new install.");
        } else {
            message.push_str("If you are certain that FFMPEG is installed on your OS and accessible in your PATH, please let hydrus_dev know, as this problem is likely due to an environment issue. You may be able to solve this problem immediately by putting a static build of the ffmpeg executable in your install_dir/bin folder.");
        }

        message.push_str("\n\n");
        message.push_str("You can check your current FFMPEG status through help->about.");

        hydrus_data::show_text(&message);
    }

    HydrusException::new(
        ErrorKind::Executable,
        format!(
            "Cannot interact with media because FFMPEG not found--are you sure it is installed? Full error: {error}"
        ),
    )
}

/// `handle_ffmpeg_no_content_and_generate_exception( path, stdout, stderr )`
pub fn handle_ffmpeg_no_content_and_generate_exception(
    path: impl AsRef<Path>,
    stdout: &str,
    stderr: &str,
) -> HydrusException {
    if !FFMPEG_NO_CONTENT_ERROR_PUBBED.swap(true, Ordering::Relaxed) {
        let message = format!(
            "FFMPEG, which hydrus uses to parse and render some media, and here was trying to look at \"{}\", did not return any data on a recent file metadata check! More debug info has been written to the log.",
            path.as_ref().display()
        );

        hydrus_data::show_text(&message);

        let debug_message = format!(
            "{message}\n\nSTDOUT Response: {stdout}\n\nSTDERR Response: {stderr}"
        );

        hydrus_data::debug_print(debug_message);

        hydrus_environment::dump_env(None);
    }

    HydrusException::new(
        ErrorKind::DataMissing,
        "Cannot interact with media because FFMPEG did not return any content.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_lines_are_classified() {
        let empty: Vec<String> = Vec::new();
        assert!(check_ffmpeg_error(&empty)
            .unwrap_err()
            .is_a(ErrorKind::DamagedOrUnusualFile));

        let missing = vec!["x.mp4: No such file or directory".to_string()];
        assert!(check_ffmpeg_error(&missing)
            .unwrap_err()
            .is_a(ErrorKind::FileMissing));

        let invalid = vec!["Invalid data found when processing input".to_string()];
        assert!(check_ffmpeg_error(&invalid)
            .unwrap_err()
            .is_a(ErrorKind::DamagedOrUnusualFile));

        let fine = vec!["Input #0, mov, from 'x.mp4':".to_string()];
        assert!(check_ffmpeg_error(&fine).is_ok());
    }

    #[test]
    fn the_info_command_can_include_a_manual_frame_count_pass() {
        let plain = build_info_command("x.mp4", None);

        assert_eq!(plain[1], "-xerror");
        assert_eq!(plain[2], "-i");
        assert_eq!(plain[3], "x.mp4");
        assert_eq!(plain.len(), 4);

        let counted = build_info_command("x.mp4", Some("0:1"));

        assert!(counted.windows(2).any(|pair| pair == ["-map", "0:1"]));
        assert!(counted.windows(2).any(|pair| pair == ["-vf", "scale=-2:120"]));
        assert_eq!(
            counted.last().map(String::as_str),
            Some(if PLATFORM_WINDOWS { "NUL" } else { "/dev/null" })
        );
    }

    #[test]
    fn ffmpeg_path_or_fallback_name_is_never_empty() {
        assert!(!get_current_ffmpeg_path().is_empty());
        assert!(!FFMPEG_EXE_NAME.is_empty());
        assert_eq!(FFMPEG_SUBPROCESS_TIMEOUT, 15);
    }
}
