use std::path::{Path, PathBuf};

fn main() {
    configure_qt_environment();

    // The C++ sources are UTF-8; the MSVC default is the system code page, which
    // would turn literals like "·" into mojibake on a non-UTF-8 Windows locale.
    unsafe {
        cxx_qt_build::CxxQtBuilder::new()
            .qt_module("Widgets")
            .qt_module("Svg")
            .file("src/gui/qt/bridge.rs")
            .include_dir("src/gui/qt")
            .cpp_file("src/gui/qt/bridge.cpp")
            .cpp_files([
                "src/gui/qt/support/strings.cpp",
                "src/gui/qt/support/icons.cpp",
                "src/gui/qt/support/widgets.cpp",
                "src/gui/qt/support/pairs.cpp",
                "src/gui/qt/support/sort_options.cpp",
                "src/gui/qt/canvas.cpp",
                "src/gui/qt/metadata_panel.cpp",
                "src/gui/qt/autocomplete.cpp",
                "src/gui/qt/viewer.cpp",
                "src/gui/qt/notebook.cpp",
                "src/gui/qt/thumbnail_grid.cpp",
                "src/gui/qt/thumbnail_grid_dnd.cpp",
                "src/gui/qt/query_page_sidebar.cpp",
                "src/gui/qt/query_page_results.cpp",
                "src/gui/qt/query_page_state.cpp",
                "src/gui/qt/downloader_page_layout.cpp",
                "src/gui/qt/downloader_page_gallery.cpp",
                "src/gui/qt/downloader_page_importers.cpp",
                "src/gui/qt/downloader_page_state.cpp",
                "src/gui/qt/duplicates_page_sidebar.cpp",
                "src/gui/qt/duplicates_page_media.cpp",
            ])
            .cc_builder(|cc| {
                cc.flag_if_supported("/utf-8");
                cc.flag_if_supported("-finput-charset=UTF-8");
                cc.flag_if_supported("-fexec-charset=UTF-8");
            })
            .build();
    }
}

/// Locate the Qt installation used by this checkout. An explicit QMAKE or
/// QT_ROOT from the environment wins, then qmake on PATH, then the usual
/// Windows install locations, so the build is not tied to one machine.
fn configure_qt_environment() {
    if let Some(qmake) = qmake_from_env()
        .or_else(qmake_from_path)
        .or_else(qmake_from_qt_dir)
    {
        use_qmake(&qmake);
    } else {
        println!(
            "cargo:warning=Qt not found; install Qt 6 (MSVC x64) or set QMAKE / QT_ROOT / HYDRUS_QT_BIN"
        );
    }
}

fn qmake_from_env() -> Option<PathBuf> {
    if let Some(value) = std::env::var_os("QMAKE") {
        let path = PathBuf::from(value);
        if path.is_file() {
            return Some(path);
        }
    }

    let root = PathBuf::from(std::env::var_os("QT_ROOT")?);

    ["bin/qmake.exe", "bin/qmake"]
        .iter()
        .map(|relative| root.join(relative))
        .find(|path| path.is_file())
}

fn qmake_from_path() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;

    std::env::split_paths(&path)
        .flat_map(|directory| {
            ["qmake.exe", "qmake"]
                .iter()
                .map(move |name| directory.join(name))
        })
        .find(|candidate| candidate.is_file())
}

fn qmake_from_qt_dir() -> Option<PathBuf> {
    let mut installs: Vec<(Vec<u32>, PathBuf)> = Vec::new();

    for entry in std::fs::read_dir(r"C:\Qt").ok()? {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };

        let mut version = Vec::new();
        let mut valid = true;
        for part in name.split('.') {
            match part.parse::<u32>() {
                Ok(number) => version.push(number),
                Err(_) => {
                    valid = false;
                    break;
                }
            }
        }
        if !valid || version.is_empty() {
            continue;
        }

        installs.push((version, entry.path()));
    }

    installs.sort_by(|left, right| right.0.cmp(&left.0));

    for (_, install) in installs {
        for kit in ["msvc2022_64", "msvc2019_64", "msvc2017_64"] {
            let qmake = install.join(kit).join("bin").join("qmake.exe");
            if qmake.is_file() {
                return Some(qmake);
            }
        }
    }

    None
}

fn use_qmake(qmake: &Path) {
    // SAFETY: build scripts are single-threaded at this point.
    unsafe { std::env::set_var("QMAKE", qmake) };

    if let Some(prefix) = qmake.parent().and_then(Path::parent) {
        // SAFETY: see above.
        unsafe { std::env::set_var("CMAKE_PREFIX_PATH", prefix) };
    }
}
