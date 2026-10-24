//! Rust port of `hydrus/core/processes/HydrusProcess.py`.
//!
//! Python's psutil gives process start times and listening sockets. The Rust
//! port does not pull in a process-inspection crate yet, so [`PSUTIL_OK`]
//! remains `false` (see [`crate::hydrus_psutil`]) and the same early exits
//! apply: the running-file checks are skipped, and asking for sibling ports
//! raises `CancelledException` exactly where Python would.
//!
//! `restart_process` uses `exec` on Unix and spawn-then-exit on Windows, which
//! is the closest equivalent to Python's `os.execv`.

use std::ffi::OsString;
use std::path::Path;

use crate::hydrus_constants::running_from_source;
use crate::hydrus_data::{print_exception, safe_print};
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_psutil::PSUTIL_OK;
use crate::hydrus_text::deserialise_newlined_texts;

fn running_file_path(db_path: &Path, instance: &str) -> std::path::PathBuf {
    db_path.join(format!("{instance}_running"))
}

/// `get_sibling_process_ports( db_path, instance )`
pub fn get_sibling_process_ports(db_path: &Path, instance: &str) -> HydrusResult<Option<Vec<u16>>> {
    let path = running_file_path(db_path, instance);

    if !path.exists() {
        return Ok(None);
    }

    let file_text = std::fs::read_to_string(&path).map_err(|e| {
        HydrusException::new(
            crate::hydrus_exceptions::ErrorKind::FileMissing,
            format!(
                "Could not read the running file \"{}\": {e}",
                path.display()
            ),
        )
    })?;

    let texts = deserialise_newlined_texts(&file_text);

    let (pid_text, _create_time_text) = match (texts.first(), texts.get(1)) {
        (Some(pid), Some(create_time)) => (pid, create_time),
        _ => return Ok(None),
    };

    let pid = match pid_text.parse::<u32>() {
        Ok(pid) => pid,
        Err(_) => return Ok(None),
    };

    if !PSUTIL_OK {
        return Err(HydrusException::new(
            ErrorKind::Cancelled,
            "psutil is not available--cannot determine sibling process ports!",
        ));
    }

    // TODO: enumerate the process's listening sockets once a process
    // inspection crate is available. Python returns the local ports of every
    // connection in the `LISTEN` state.
    let _ = pid;

    Ok(None)
}

fn time_matches(process_time: f64, recorded_time: f64) -> bool {
    recorded_time - 0.5 < process_time && process_time < recorded_time + 0.5
}

/// `is_already_running( db_path, instance )`
pub fn is_already_running(db_path: &Path, instance: &str) -> bool {
    if !PSUTIL_OK {
        safe_print("psutil is not available, so cannot do the \"already running?\" check!");

        return false;
    }

    let path = running_file_path(db_path, instance);

    if !path.exists() {
        return false;
    }

    let file_text = match std::fs::read_to_string(&path) {
        Ok(file_text) => file_text,
        Err(e) => {
            safe_print("Problem loading the already-running file:");
            safe_print(print_exception(
                &HydrusException::new(
                    crate::hydrus_exceptions::ErrorKind::FileMissing,
                    e.to_string(),
                ),
                false,
            ));

            return false;
        }
    };

    let texts = deserialise_newlined_texts(&file_text);

    let (pid_text, create_time_text) = match (texts.first(), texts.get(1)) {
        (Some(pid), Some(create_time)) => (pid, create_time),
        _ => return false,
    };

    let Ok(pid) = pid_text.parse::<u32>() else {
        return false;
    };

    let Ok(recorded_create_time) = create_time_text.parse::<f64>() else {
        return false;
    };

    // The Rust port cannot query process creation times without psutil, so
    // even if the capability flag were flipped this check would need one more
    // implementation. Keep the timestamp compare here so the logic is ready.
    let _ = time_matches;

    // `me.pid == pid` cannot be checked here because there is no creation-time
    // query. A Linux `os.execl()` restart keeps the pid, so the self check is
    // important once psutil is available.
    let _ = (pid, recorded_create_time);

    false
}

/// `record_running_start( db_path, instance )`
pub fn record_running_start(db_path: &Path, instance: &str) {
    if !PSUTIL_OK {
        return;
    }

    // TODO: write `<pid>\n<create_time>` once process creation times are
    // available. As with Python, a failure to query the process is ignored.
    let _ = (db_path, instance);
}

/// `restart_process()`
pub fn restart_process() -> HydrusResult<()> {
    // Time for ports to unmap.
    std::thread::sleep(std::time::Duration::from_secs(1));

    let exe = std::env::current_exe().map_err(|e| {
        HydrusException::new(
            crate::hydrus_exceptions::ErrorKind::Executable,
            format!("Could not find the executable to restart: {e}"),
        )
    })?;

    let mut args: Vec<OsString> = Vec::new();

    if running_from_source() {
        // exe is the interpreter, me is the script. Rust binaries usually
        // restart themselves, so keep argv intact.
        args.push(exe.clone().into_os_string());
        args.extend(std::env::args_os());
    } else {
        args.push(exe.clone().into_os_string());
        args.extend(std::env::args_os().skip(1));
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;

        let error = std::process::Command::new(&exe).args(&args[1..]).exec();

        Err(HydrusException::new(
            crate::hydrus_exceptions::ErrorKind::Executable,
            format!("Could not exec the restarted process: {error}"),
        ))
    }

    #[cfg(not(unix))]
    {
        let status = std::process::Command::new(&exe)
            .args(&args[1..])
            .spawn()
            .map_err(|e| {
                HydrusException::new(
                    crate::hydrus_exceptions::ErrorKind::Executable,
                    format!("Could not start the restarted process: {e}"),
                )
            })?;

        // Spawning is asynchronous; give the child a moment to replace us.
        let _ = status;

        std::process::exit(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(tag: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "hydrus-process-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        std::fs::create_dir_all(&path).unwrap();

        path
    }

    #[test]
    fn no_running_file_means_no_sibling_ports() {
        let dir = unique_dir("none");

        assert_eq!(get_sibling_process_ports(&dir, "client").unwrap(), None);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn malformed_running_file_is_ignored() {
        let dir = unique_dir("malformed");

        std::fs::write(dir.join("client_running"), "not-a-pid\nnope").unwrap();

        assert_eq!(get_sibling_process_ports(&dir, "client").unwrap(), None);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn parseable_running_file_requires_psutil() {
        let dir = unique_dir("valid");

        std::fs::write(dir.join("client_running"), "1234\n1700000000.5").unwrap();

        // PSUTIL_OK is false in this port, so Python's next step raises.
        let error = get_sibling_process_ports(&dir, "client").err().unwrap();

        assert!(error.is_a(crate::hydrus_exceptions::ErrorKind::Cancelled));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn already_running_is_false_without_psutil() {
        let dir = unique_dir("already-running");

        std::fs::write(dir.join("client_running"), "1234\n1700000000.5").unwrap();

        assert!(!is_already_running(&dir, "client"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn record_running_start_is_a_no_op_without_psutil() {
        let dir = unique_dir("record");

        record_running_start(&dir, "client");

        assert!(!dir.join("client_running").exists());

        let _ = std::fs::remove_dir_all(dir);
    }
}
