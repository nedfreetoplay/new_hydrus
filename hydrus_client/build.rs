use std::path::Path;

fn main() {
    configure_qt_environment();

    cxx_qt_build::CxxQtBuilder::new()
        .qt_module("Widgets")
        .qt_module("Svg")
        .file("src/gui/qt/bridge.rs")
        .include_dir("src/gui/qt")
        .cpp_file("src/gui/qt/bridge.cpp")
        .build();
}

/// Locate the Qt installation used by this checkout when the caller did not
/// provide one explicitly. Keeping the lookup here means `cargo build` works
/// from a plain shell on the development machine.
fn configure_qt_environment() {
    if std::env::var_os("QMAKE").is_some() {
        return;
    }

    let candidates = [
        r"C:\Qt\6.12.0\msvc2022_64\bin\qmake.exe",
        r"C:\Qt\6.11.3\msvc2022_64\bin\qmake.exe",
        r"C:\Qt\6.10.3\msvc2022_64\bin\qmake.exe",
    ];

    for candidate in candidates {
        if Path::new(candidate).exists() {
            // SAFETY: build scripts are single-threaded at this point.
            unsafe { std::env::set_var("QMAKE", candidate) };
            if let Some(prefix) = Path::new(candidate).parent().and_then(Path::parent) {
                unsafe { std::env::set_var("CMAKE_PREFIX_PATH", prefix) };
            }
            return;
        }
    }
}
