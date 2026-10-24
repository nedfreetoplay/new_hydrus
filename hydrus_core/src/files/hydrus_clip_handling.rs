//! Rust port of `hydrus/core/files/HydrusClipHandling.py`.
//!
//! Clip Studio Paint `.clip` files are a proprietary wrapper around an SQLite
//! database. We locate the database by its magic string, copy it to a temp
//! file, and read the `Canvas`, `CanvasPreview` and `TimeLine` tables.

use std::io::Write;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension};

use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_temp;

const SQLITE_START: &[u8] = b"SQLite format 3";

fn damaged(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::DamagedOrUnusualFile, message)
}

fn no_thumbnail() -> HydrusException {
    HydrusException::new(ErrorKind::NoThumbnailFile, "")
}

/// `get_sqlite_db( path, sqlite_temp_path )`
///
/// Python returns a `( db, c )` pair. Rust's `Connection` owns its cursors, so
/// only the connection is returned.
pub fn get_sqlite_db(
    path: impl AsRef<Path>,
    sqlite_temp_path: impl AsRef<Path>,
) -> HydrusResult<Connection> {
    let clip_bytes = std::fs::read(path.as_ref()).map_err(|error| {
        HydrusException::new(
            ErrorKind::FileMissing,
            format!("Could not read the clip file: {error}"),
        )
    })?;

    let Some(index) = clip_bytes
        .windows(SQLITE_START.len())
        .position(|window| window == SQLITE_START)
    else {
        return Err(damaged("This clip file had no internal SQLite file!"));
    };

    let sqlite_bytes = &clip_bytes[index..];

    std::fs::write(sqlite_temp_path.as_ref(), sqlite_bytes).map_err(|error| {
        damaged(format!(
            "Could not write the internal SQLite file to the temp path: {error}"
        ))
    })?;

    Connection::open(sqlite_temp_path.as_ref())
        .map_err(|_| damaged("This clip file seemed to have an invalid internal SQLite file!"))
}

fn with_clip_db<T>(path: &Path, f: impl FnOnce(&Connection) -> HydrusResult<T>) -> HydrusResult<T> {
    let (file_handle, sqlite_temp_path) = hydrus_temp::get_temp_path("clip_sqlite", "", None)?;

    let result = (|| {
        let db = get_sqlite_db(path, &sqlite_temp_path)?;
        f(&db)
    })();

    hydrus_temp::clean_up_temp_path(file_handle, &sqlite_temp_path);

    result
}

/// `extract_db_png_to_path( path, temp_path )`
pub fn extract_db_png_to_path(
    path: impl AsRef<Path>,
    temp_path: impl AsRef<Path>,
) -> HydrusResult<()> {
    let result = with_clip_db(path.as_ref(), |db| {
        let png_bytes: Vec<u8> = db
            .query_row("SELECT ImageData FROM CanvasPreview;", [], |row| row.get(0))
            .map_err(|error| {
                HydrusException::new(
                    ErrorKind::DamagedOrUnusualFile,
                    format!("Could not read the clip preview: {error}"),
                )
            })?;

        let mut output = std::fs::File::create(temp_path.as_ref()).map_err(|error| {
            HydrusException::new(
                ErrorKind::NoThumbnailFile,
                format!("Could not write the clip preview: {error}"),
            )
        })?;

        output.write_all(&png_bytes).map_err(|error| {
            HydrusException::new(
                ErrorKind::NoThumbnailFile,
                format!("Could not write the clip preview: {error}"),
            )
        })?;

        Ok(())
    });

    result.map_err(|_| no_thumbnail())
}

/// `get_clip_properties( path )`
///
/// Returns `( ( width, height ), duration_ms, num_frames )`. The duration is a
/// float because Python computes `num_frames / framerate * 1000`.
pub fn get_clip_properties(
    path: impl AsRef<Path>,
) -> HydrusResult<((i64, i64), Option<f64>, Option<i64>)> {
    with_clip_db(path.as_ref(), |db| {
        let (width_float, height_float, canvas_unit, canvas_dpi_float): (f64, f64, i64, f64) = db
            .query_row(
                "SELECT CanvasWidth, CanvasHeight, CanvasUnit, CanvasResolution FROM Canvas;",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(|error| {
                damaged(format!(
                    "Could not read the clip canvas properties: {error}"
                ))
            })?;

        let mut num_frames: Option<i64> = None;
        let mut duration_ms: Option<f64> = None;

        let has_timeline: Option<i64> = db
            .query_row(
                r#"SELECT 1 FROM sqlite_master WHERE name = "TimeLine";"#,
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| damaged(format!("Could not inspect the clip database: {error}")))?;

        if has_timeline.is_some() {
            let timeline: Option<(f64, f64, f64)> = db
                .query_row(
                    "SELECT StartFrame, FrameRate, EndFrame from TimeLine;",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(|error| damaged(format!("Could not read the clip timeline: {error}")))?;

            if let Some((start_frame_float, mut framerate_float, end_frame_float)) = timeline {
                let frames = (end_frame_float - start_frame_float) as i64;

                if framerate_float == 0.0 {
                    framerate_float = 24.0;
                }

                num_frames = Some(frames);
                duration_ms = Some((frames as f64 / framerate_float) * 1000.0);
            }
        }

        // Width and height are in canvas units; the resolution is always DPI.
        let unit_conversion_multiplier = match canvas_unit {
            0 => 1.0,                     // pixels
            1 => canvas_dpi_float / 2.54, // cm
            2 => canvas_dpi_float / 25.4, // mm
            3 => canvas_dpi_float,        // inches
            5 => canvas_dpi_float / 72.0, // points
            _ => 1.0,
        };

        Ok((
            (
                (width_float * unit_conversion_multiplier).round() as i64,
                (height_float * unit_conversion_multiplier).round() as i64,
            ),
            duration_ms,
            num_frames,
        ))
    })
}

fn unique_temp_path(label: &str, extension: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "hydrus_clip_{}_{}_{}{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        extension
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    fn build_clip(
        label: &str,
        canvas: &str,
        preview: Option<&[u8]>,
        timeline: Option<&str>,
    ) -> PathBuf {
        let db_path = unique_temp_path(label, ".db");
        let clip_path = unique_temp_path(label, ".clip");

        {
            let db = Connection::open(&db_path).unwrap();

            db.execute_batch(canvas).unwrap();

            if let Some(timeline) = timeline {
                db.execute_batch(timeline).unwrap();
            }

            if let Some(preview) = preview {
                db.execute("CREATE TABLE CanvasPreview ( ImageData BLOB );", [])
                    .unwrap();
                db.execute(
                    "INSERT INTO CanvasPreview ( ImageData ) VALUES ( ?1 );",
                    params![preview],
                )
                .unwrap();
            }
        }

        let sqlite_bytes = std::fs::read(&db_path).unwrap();

        let mut clip_bytes = b"CLIP STUDIO PAINT FILE".to_vec();
        clip_bytes.extend_from_slice(&sqlite_bytes);

        std::fs::write(&clip_path, clip_bytes).unwrap();

        std::fs::remove_file(db_path).unwrap();

        clip_path
    }

    #[test]
    fn canvas_properties_are_in_pixels_for_unit_zero() {
        let path = build_clip(
            "pixels",
            "CREATE TABLE Canvas ( CanvasWidth REAL, CanvasHeight REAL, CanvasUnit INTEGER, CanvasResolution REAL );
             INSERT INTO Canvas VALUES ( 100.4, 50.6, 0, 72.0 );",
            None,
            None,
        );

        let ((width, height), duration, frames) = get_clip_properties(&path).unwrap();

        assert_eq!((width, height), (100, 51));
        assert_eq!(duration, None);
        assert_eq!(frames, None);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn timeline_gives_frames_and_duration() {
        let path = build_clip(
            "timeline",
            "CREATE TABLE Canvas ( CanvasWidth REAL, CanvasHeight REAL, CanvasUnit INTEGER, CanvasResolution REAL );
             INSERT INTO Canvas VALUES ( 10, 20, 0, 72.0 );",
            None,
            Some(
                "CREATE TABLE TimeLine ( StartFrame REAL, FrameRate REAL, EndFrame REAL );
                 INSERT INTO TimeLine VALUES ( 5, 10, 55 );",
            ),
        );

        let ((width, height), duration, frames) = get_clip_properties(&path).unwrap();

        assert_eq!((width, height), (10, 20));
        assert_eq!(frames, Some(50));
        assert_eq!(duration, Some(5000.0));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn centimetres_are_converted_through_dpi() {
        let path = build_clip(
            "cm",
            "CREATE TABLE Canvas ( CanvasWidth REAL, CanvasHeight REAL, CanvasUnit INTEGER, CanvasResolution REAL );
             INSERT INTO Canvas VALUES ( 10, 5, 1, 254.0 );",
            None,
            None,
        );

        let ((width, height), _, _) = get_clip_properties(&path).unwrap();

        assert_eq!((width, height), (1000, 500));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn preview_bytes_are_extracted() {
        let path = build_clip(
            "preview",
            "CREATE TABLE Canvas ( CanvasWidth REAL, CanvasHeight REAL, CanvasUnit INTEGER, CanvasResolution REAL );
             INSERT INTO Canvas VALUES ( 10, 10, 0, 72.0 );",
            Some(b"PNGDATA"),
            None,
        );

        let output = unique_temp_path("preview_out", ".png");

        extract_db_png_to_path(&path, &output).unwrap();

        assert_eq!(std::fs::read(&output).unwrap(), b"PNGDATA");

        std::fs::remove_file(path).unwrap();
        std::fs::remove_file(output).unwrap();
    }

    #[test]
    fn missing_sqlite_marker_is_damaged() {
        let path = unique_temp_path("bad", ".clip");
        std::fs::write(&path, b"no database here").unwrap();

        let error = get_sqlite_db(&path, unique_temp_path("bad_db", ".db")).unwrap_err();

        assert!(error.is_a(ErrorKind::DamagedOrUnusualFile));

        std::fs::remove_file(path).unwrap();
    }
}
