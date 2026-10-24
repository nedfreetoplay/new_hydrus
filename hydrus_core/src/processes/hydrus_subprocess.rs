//! Rust port of `hydrus/core/processes/HydrusSubprocess.py`.
//!
//! Python's `subprocess.Popen` maps to [`std::process::Command`]. The port
//! keeps the same pipeline:
//!
//! * a shared "make a subprocess" kwargs bundle (environment, text mode,
//!   hidden terminal, shell);
//! * a long-lived process registry that is reaped periodically;
//! * timeout/shutdown-aware `communicate` (implemented by polling
//!   `Child::try_wait` while reader threads drain the pipes);
//! * the streaming [`SubprocessContext`] used by ffmpeg.
//!
//! `bufsize` has no Rust equivalent and is accepted only for API parity.
//! Output is returned as bytes; use [`SubprocessResult::stdout_text`] when
//! `text = true` would have been used in Python.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::Read;
use std::process::{Child, Command as StdCommand, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use crate::hydrus_boot;
use crate::hydrus_constants::{
    PLATFORM_LINUX, PLATFORM_MACOS, PLATFORM_WINDOWS, running_from_frozen_build,
};
use crate::hydrus_data::{
    check_program_is_not_shutting_down, print_exception, safe_print as show_text,
};
use crate::hydrus_environment;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult, shutdown};
use crate::hydrus_globals;
use crate::hydrus_numbers::to_human_int;
use crate::hydrus_time::{get_now_float, time_has_passed_float};

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

/// A command line, mirroring Python's list-or-string `cmd` argument.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SubprocessCommand {
    /// `[ exe, '--flag', '/path' ]`
    Args(Vec<String>),
    /// A string handed to the platform shell.
    Shell(String),
}

impl SubprocessCommand {
    pub fn args<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::Args(args.into_iter().map(Into::into).collect())
    }

    pub fn shell(cmd: impl Into<String>) -> Self {
        Self::Shell(cmd.into())
    }

    fn to_std_command(&self) -> HydrusResult<StdCommand> {
        match self {
            Self::Args(args) if args.is_empty() => Err(HydrusException::new(
                ErrorKind::Executable,
                "Cannot run an empty command!",
            )),
            Self::Args(args) => {
                let mut command = StdCommand::new(&args[0]);

                command.args(&args[1..]);

                Ok(command)
            }
            Self::Shell(cmd) => {
                #[cfg(windows)]
                let command = {
                    let mut command = StdCommand::new("cmd");

                    command.arg("/C").arg(cmd);

                    command
                };

                #[cfg(not(windows))]
                let command = {
                    let mut command = StdCommand::new("/bin/sh");

                    command.arg("-c").arg(cmd);

                    command
                };

                Ok(command)
            }
        }
    }
}

impl std::fmt::Display for SubprocessCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Args(args) => write!(formatter, "{args:?}"),
            Self::Shell(cmd) => write!(formatter, "{cmd}"),
        }
    }
}

/// `get_subprocess_env()`
///
/// Returns `None` when the inherited environment can be used unchanged, as in
/// Python.
pub fn get_subprocess_env() -> Option<std::collections::HashMap<OsString, OsString>> {
    if hydrus_globals::subprocess_report_mode() {
        hydrus_environment::dump_env(None);
    }

    let mut env: std::collections::HashMap<OsString, OsString> = std::env::vars_os().collect();

    if let Some(original_path) = hydrus_boot::original_path() {
        env.insert(OsString::from("PATH"), original_path);
    }

    if !running_from_frozen_build() {
        return None;
    }

    let mut changes_made = false;

    let orig_swaperoo_strings = ["LD_LIBRARY_PATH", "XDG_DATA_DIRS"];
    let ok_to_remove_absent_orig = ["LD_LIBRARY_PATH"];

    for key in orig_swaperoo_strings {
        let orig_key = format!("{key}_ORIG");

        if let Some(original) = env.get(OsString::from(&orig_key).as_os_str()).cloned() {
            env.insert(OsString::from(key), original);

            changes_made = true;
        } else if env.contains_key(OsString::from(key).as_os_str())
            && ok_to_remove_absent_orig.contains(&key)
        {
            env.remove(OsString::from(key).as_os_str());

            changes_made = true;
        }
    }

    let remove_if_hydrus_base_dir = ["QT_PLUGIN_PATH", "QML2_IMPORT_PATH", "SSL_CERT_FILE"];

    if let Some(hydrus_base_dir) = hydrus_globals::db_dir() {
        let hydrus_base_dir = hydrus_base_dir.to_string_lossy().to_string();

        for key in remove_if_hydrus_base_dir {
            let key = OsString::from(key);

            if let Some(value) = env.get(&key) {
                if value.to_string_lossy().starts_with(&hydrus_base_dir) {
                    env.remove(&key);

                    changes_made = true;
                }
            }
        }
    }

    if PLATFORM_LINUX || PLATFORM_MACOS {
        if let Some(path) = env.get(OsString::from("PATH").as_os_str()).cloned() {
            let path = path.to_string_lossy().to_string();

            let mut path_locations: std::collections::HashSet<String> =
                path.split(':').map(str::to_string).collect();

            let mut path = path;

            for desired in ["/usr/bin", "/usr/local/bin"] {
                if !path_locations.contains(desired) {
                    path = format!("{desired}:{path}");
                    path_locations.insert(desired.to_string());

                    changes_made = true;
                }
            }

            env.insert(OsString::from("PATH"), OsString::from(path));
        }

        if let Some(xdg_data_dirs) = env.get(OsString::from("XDG_DATA_DIRS").as_os_str()) {
            let xdg_data_dirs = xdg_data_dirs.to_string_lossy().to_string();
            let base_dir = crate::hydrus_constants::base_dir()
                .to_string_lossy()
                .to_string();
            let content_base_dir = crate::hydrus_constants::content_base_dir()
                .to_string_lossy()
                .to_string();

            if !xdg_data_dirs.contains(':')
                && (xdg_data_dirs.contains(&base_dir) || xdg_data_dirs.contains(&content_base_dir))
            {
                env.insert(
                    OsString::from("XDG_DATA_DIRS"),
                    OsString::from("/usr/local/share:/usr/share"),
                );

                changes_made = true;
            }
        }
    }

    if changes_made { Some(env) } else { None }
}

/// `get_subprocess_hide_terminal_startup_info()`
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StartupInfo {
    pub hide_terminal: bool,
}

pub fn get_subprocess_hide_terminal_startup_info() -> StartupInfo {
    StartupInfo {
        hide_terminal: PLATFORM_WINDOWS,
    }
}

/// `get_subprocess_kw_args()`
#[derive(Clone, Debug)]
pub struct SubprocessKwArgs {
    pub env: Option<std::collections::HashMap<OsString, OsString>>,
    pub text: bool,
    pub shell: bool,
    pub startup_info: StartupInfo,
}

impl SubprocessKwArgs {
    fn apply(&self, command: &mut StdCommand) {
        if let Some(env) = &self.env {
            command.env_clear();
            command.envs(env);
        }

        #[cfg(windows)]
        if self.startup_info.hide_terminal {
            use std::os::windows::process::CommandExt as _;

            const CREATE_NO_WINDOW: u32 = 0x0800_0000;

            command.creation_flags(CREATE_NO_WINDOW);
        }
    }
}

pub fn get_subprocess_kw_args(hide_terminal: bool, text: bool, shell: bool) -> SubprocessKwArgs {
    let mut startup_info = get_subprocess_hide_terminal_startup_info();

    startup_info.hide_terminal = startup_info.hide_terminal && hide_terminal;

    let kwargs = SubprocessKwArgs {
        env: get_subprocess_env(),
        text,
        shell,
        startup_info,
    };

    if hydrus_globals::subprocess_report_mode() {
        show_text(format!("KWargs are: {kwargs:?}"));
    }

    kwargs
}

// ---------------------------------------------------------------------------
// Long-lived processes
// ---------------------------------------------------------------------------

static LONG_LIVED_EXTERNAL_PROCESSES: Mutex<Option<Vec<Child>>> = Mutex::new(None);

/// `reap_dead_long_lived_external_processes()`
pub fn reap_dead_long_lived_external_processes() {
    let mut processes = lock(&LONG_LIVED_EXTERNAL_PROCESSES);

    let Some(processes) = processes.as_mut() else {
        return;
    };

    processes.retain_mut(|process| match process.try_wait() {
        Ok(Some(_)) => false,
        Ok(None) => true,
        Err(_) => false,
    });
}

/// `register_long_lived_external_process( process )`
pub fn register_long_lived_external_process(process: Child) {
    lock(&LONG_LIVED_EXTERNAL_PROCESSES)
        .get_or_insert_with(Vec::new)
        .push(process);
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

fn preview(bytes: Option<&[u8]>) -> String {
    match bytes {
        None => "no content".to_string(),
        Some(bytes) => {
            let limit = bytes.len().min(256);

            format!("{:?}", String::from_utf8_lossy(&bytes[..limit]))
        }
    }
}

/// `report_bad_return_code( cmd, returncode, stdout, stderr )`
pub fn report_bad_return_code(
    cmd: &SubprocessCommand,
    returncode: i32,
    stdout: Option<&[u8]>,
    stderr: Option<&[u8]>,
) -> HydrusException {
    let message = format!(
        "A call to another executable gave a non-zero return code ({returncode})! The call was: {cmd}\n\n\
         ========== stdout ==========\n{}\n\
         ========== stderr ==========\n{}\n\
         ============================",
        preview(stdout),
        preview(stderr)
    );

    HydrusException::new(ErrorKind::BadReturnCode, message)
}

/// `report_timeout_error( cmd, timeout, stdout, stderr )`
pub fn report_timeout_error(
    cmd: &SubprocessCommand,
    timeout: u64,
    stdout: Option<&[u8]>,
    stderr: Option<&[u8]>,
) -> HydrusException {
    let message = format!(
        "A call to another executable took too long (over {} seconds) to finish! The call was: {cmd}\n\n\
         ========== stdout ==========\n{}\n\
         ========== stderr ==========\n{}\n\
         ============================",
        to_human_int(timeout as i128),
        preview(stdout),
        preview(stderr)
    );

    HydrusException::new(ErrorKind::SubprocessTimedOut, message)
}

// ---------------------------------------------------------------------------
// Spawning and communicating
// ---------------------------------------------------------------------------

/// `run_subprocess_raw_call(...)`
#[allow(clippy::too_many_arguments)]
pub fn run_subprocess_raw_call(
    cmd: &SubprocessCommand,
    start_new_session: bool,
    _bufsize: usize,
    stdin_pipe: bool,
    stdout_pipe: bool,
    stderr_pipe: bool,
    hide_terminal: bool,
    text: bool,
    shell: bool,
) -> HydrusResult<Child> {
    let kwargs = get_subprocess_kw_args(hide_terminal, text, shell);

    let mut command = cmd.to_std_command()?;

    command.stdin(if stdin_pipe {
        Stdio::piped()
    } else {
        Stdio::null()
    });

    command.stdout(if stdout_pipe {
        Stdio::piped()
    } else {
        Stdio::null()
    });

    command.stderr(if stderr_pipe {
        Stdio::piped()
    } else {
        Stdio::null()
    });

    #[cfg(unix)]
    if start_new_session {
        use std::os::unix::process::CommandExt as _;

        command.process_group(0);
    }

    #[cfg(not(unix))]
    let _ = start_new_session;

    kwargs.apply(&mut command);

    match command.spawn() {
        Ok(child) => Ok(child),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            show_text(format!(
                "Got a file not found on this external program call: {cmd}"
            ));
            show_text(format!(
                "If the error is not obvious, you might want to talk to hydev about it. Maybe your env PATH is unusual. Your env will follow, and here were the kwarg settings used in the subprocess call: {kwargs:?}"
            ));
            hydrus_environment::dump_env(None);

            Err(HydrusException::new(
                ErrorKind::Executable,
                format!("External program not found: {cmd}"),
            ))
        }
        Err(e) => {
            show_text(format!(
                "Had a problem with an external program! Error will follow; command was: {cmd}"
            ));
            show_text(print_exception(
                &HydrusException::new(ErrorKind::Executable, e.to_string()),
                false,
            ));

            Err(HydrusException::new(ErrorKind::Executable, e.to_string()))
        }
    }
}

/// `subprocess_communicate( cmd, process, timeout )`
///
/// Returns `( stdout, stderr )`. The pipes are read by helper threads so the
/// timeout and shutdown checks stay responsive.
pub fn subprocess_communicate(
    cmd: &SubprocessCommand,
    process: &mut Child,
    timeout: u64,
) -> HydrusResult<(Vec<u8>, Vec<u8>)> {
    if let Err(e) = check_program_is_not_shutting_down() {
        let _ = process.kill();
        let _ = process.wait();

        return Err(e);
    }

    // Closing stdin sends EOF to programs that wait for it.
    drop(process.stdin.take());

    let stdout = process.stdout.take();
    let stderr = process.stderr.take();

    let stdout_reader = stdout.map(|mut stdout| {
        std::thread::spawn(move || {
            let mut buffer = Vec::new();

            let _ = stdout.read_to_end(&mut buffer);

            buffer
        })
    });

    let stderr_reader = stderr.map(|mut stderr| {
        std::thread::spawn(move || {
            let mut buffer = Vec::new();

            let _ = stderr.read_to_end(&mut buffer);

            buffer
        })
    });

    let time_started = get_now_float();

    loop {
        if hydrus_globals::model_shutdown() {
            let _ = process.kill();
            let _ = process.wait();

            return Err(shutdown("Application is shutting down!"));
        }

        match process.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(e) => {
                return Err(HydrusException::new(ErrorKind::Unknown, e.to_string()));
            }
        }

        if time_has_passed_float(time_started + timeout as f64) {
            terminate_and_reap_process(process);

            let stdout = stdout_reader
                .and_then(|reader| reader.join().ok())
                .unwrap_or_default();
            let stderr = stderr_reader
                .and_then(|reader| reader.join().ok())
                .unwrap_or_default();

            return Err(report_timeout_error(
                cmd,
                timeout,
                Some(&stdout),
                Some(&stderr),
            ));
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    let stdout = stdout_reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();
    let stderr = stderr_reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();

    Ok((stdout, stderr))
}

/// `terminate_and_reap_process( process )`
pub fn terminate_and_reap_process(process: &mut Child) {
    terminate(process);

    // Give the process a moment to exit after SIGTERM/terminate before killing.
    let started = std::time::Instant::now();

    while started.elapsed() < Duration::from_secs(1) {
        match process.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => return,
        }
    }

    let _ = process.kill();
    let _ = process.wait();
}

fn terminate(process: &mut Child) {
    #[cfg(windows)]
    {
        if let Err(e) = process.kill() {
            show_text(format!(
                "I tried to terminate the process {} but got {}!",
                process.id(),
                e
            ));
        }
    }

    #[cfg(unix)]
    {
        let pid = process.id() as i32;

        // SAFETY: `kill` is called with a pid obtained from a live `Child`.
        let result = unsafe { libc::kill(pid, libc::SIGTERM) };

        if result != 0 {
            let error = std::io::Error::last_os_error();

            show_text(format!(
                "I tried to terminate the process {} but got {}!",
                process.id(),
                error
            ));
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = process.kill();
    }
}

/// `run_subprocess(...)`
#[allow(clippy::too_many_arguments)]
pub fn run_subprocess(
    cmd: &SubprocessCommand,
    timeout: u64,
    bufsize: usize,
    this_is_a_potentially_long_lived_external_guy: bool,
    hide_terminal: bool,
    text: bool,
    shell: bool,
) -> HydrusResult<SubprocessResult> {
    if this_is_a_potentially_long_lived_external_guy {
        let process = run_subprocess_raw_call(
            cmd,
            true,
            bufsize,
            false,
            false,
            false,
            hide_terminal,
            text,
            shell,
        )?;

        register_long_lived_external_process(process);

        return Ok(SubprocessResult {
            stdout: None,
            stderr: None,
            returncode: 0,
        });
    }

    let mut process = run_subprocess_raw_call(
        cmd,
        false,
        bufsize,
        true,
        true,
        true,
        hide_terminal,
        text,
        shell,
    )?;

    let (stdout, stderr) = subprocess_communicate(cmd, &mut process, timeout)?;

    let returncode = process
        .try_wait()
        .ok()
        .flatten()
        .and_then(|status| status.code())
        .unwrap_or(-1);

    if hydrus_globals::subprocess_report_mode() {
        if stdout.is_empty() && stderr.is_empty() {
            show_text("No stdout or stderr came back.");
        }

        if !stdout.is_empty() {
            show_text(format!("stdout: {}", preview(Some(&stdout))));
        }

        if !stderr.is_empty() {
            show_text(format!("stderr: {}", preview(Some(&stderr))));
        }
    }

    Ok(SubprocessResult {
        stdout: Some(stdout),
        stderr: Some(stderr),
        returncode,
    })
}

/// The triple Python's `run_subprocess` returns.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SubprocessResult {
    pub stdout: Option<Vec<u8>>,
    pub stderr: Option<Vec<u8>>,
    pub returncode: i32,
}

impl SubprocessResult {
    pub fn stdout_text(&self) -> String {
        self.stdout
            .as_ref()
            .map(|bytes| String::from_utf8_lossy(bytes).to_string())
            .unwrap_or_default()
    }

    pub fn stderr_text(&self) -> String {
        self.stderr
            .as_ref()
            .map(|bytes| String::from_utf8_lossy(bytes).to_string())
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Streaming context
// ---------------------------------------------------------------------------

struct ChunkQueue {
    chunks: VecDeque<Option<Vec<u8>>>,
}

/// `class SubprocessContext`
pub struct SubprocessContext {
    cmd: SubprocessCommand,
    timeout: u64,
    process: Arc<Mutex<Child>>,
    chunk_queue: Arc<(Mutex<ChunkQueue>, Condvar)>,
    new_chunk_desired: Arc<crate::processes::hydrus_threading::Event>,
    finished: Arc<AtomicBool>,
    closed: Arc<AtomicBool>,
    reader: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl SubprocessContext {
    pub fn new(
        cmd: SubprocessCommand,
        timeout: u64,
        bufsize: usize,
        hide_terminal: bool,
        text: bool,
        shell: bool,
    ) -> HydrusResult<Self> {
        let mut process = run_subprocess_raw_call(
            &cmd,
            false,
            bufsize,
            true,
            true,
            true,
            hide_terminal,
            text,
            shell,
        )?;

        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| HydrusException::new(ErrorKind::Executable, "No stdout pipe"))?;

        let process = Arc::new(Mutex::new(process));
        let chunk_queue = Arc::new((
            Mutex::new(ChunkQueue {
                chunks: VecDeque::new(),
            }),
            Condvar::new(),
        ));
        let new_chunk_desired = Arc::new(crate::processes::hydrus_threading::Event::new());
        let finished = Arc::new(AtomicBool::new(false));
        let closed = Arc::new(AtomicBool::new(false));

        let reader = {
            let cmd = cmd.clone();
            let chunk_queue = Arc::clone(&chunk_queue);
            let new_chunk_desired = Arc::clone(&new_chunk_desired);
            let finished = Arc::clone(&finished);
            let closed = Arc::clone(&closed);
            let bufsize = bufsize;

            std::thread::Builder::new()
                .name("SubprocessContextReader".to_string())
                .spawn(move || {
                    Self::thread_reader(
                        &cmd,
                        stdout,
                        &chunk_queue,
                        &new_chunk_desired,
                        &finished,
                        &closed,
                        bufsize,
                    )
                })
                .map_err(|e| {
                    HydrusException::new(
                        ErrorKind::Unknown,
                        format!("Could not start the subprocess reader: {e}"),
                    )
                })?
        };

        Ok(Self {
            cmd,
            timeout,
            process,
            chunk_queue,
            new_chunk_desired,
            finished,
            closed,
            reader: Mutex::new(Some(reader)),
        })
    }

    fn thread_reader(
        cmd: &SubprocessCommand,
        mut stdout: std::process::ChildStdout,
        chunk_queue: &Arc<(Mutex<ChunkQueue>, Condvar)>,
        new_chunk_desired: &Arc<crate::processes::hydrus_threading::Event>,
        finished: &Arc<AtomicBool>,
        closed: &Arc<AtomicBool>,
        bufsize: usize,
    ) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            loop {
                if closed.load(Ordering::Relaxed) {
                    return;
                }

                if check_program_is_not_shutting_down().is_err() {
                    return;
                }

                let queue_is_empty = {
                    let (queue, _) = &**chunk_queue;

                    lock(queue).chunks.is_empty()
                };

                if !queue_is_empty {
                    new_chunk_desired.wait(Duration::from_secs(1));
                    new_chunk_desired.clear();

                    continue;
                }

                let mut buffer = vec![0_u8; bufsize.max(1)];
                let mut filled = 0;

                // Python's buffered `stdout.read( n )` fills the requested
                // buffer unless the stream ends. Rust's raw `Read` may return
                // a partial pipe read, and video frames must arrive as one
                // complete chunk.
                while filled < buffer.len() {
                    match stdout.read(&mut buffer[filled..]) {
                        Ok(0) => break,
                        Ok(n) => filled += n,
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {
                            show_text(format!(
                                "Probably not a big deal, but the Subprocess Command \"{cmd}\" closed its stdout early. If this keeps happening, please let hydev know!"
                            ));

                            return;
                        }
                        Err(_) => return,
                    }
                }

                if filled == 0 {
                    return;
                }

                buffer.truncate(filled);

                let (queue, condvar) = &**chunk_queue;

                lock(queue).chunks.push_back(Some(buffer));

                condvar.notify_all();
            }
        }));

        if result.is_err() {
            show_text(format!(
                "The Subprocess Command \"{cmd}\" reader thread panicked."
            ));
        }

        finished.store(true, Ordering::Relaxed);

        let (queue, condvar) = &**chunk_queue;

        lock(queue).chunks.push_back(None);

        condvar.notify_all();
    }

    /// Python's `finished` attribute.
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }

    /// `read_chunk()`
    pub fn read_chunk(&self) -> HydrusResult<Vec<u8>> {
        self.new_chunk_desired.set();

        let (queue, condvar) = &*self.chunk_queue;

        let mut queue = lock(queue);

        while queue.chunks.is_empty() {
            let (new_queue, timeout) = condvar
                .wait_timeout(queue, Duration::from_secs(self.timeout))
                .unwrap_or_else(|e| e.into_inner());

            queue = new_queue;

            if timeout.timed_out() && queue.chunks.is_empty() {
                drop(queue);

                self.terminate_and_reap();

                return Err(report_timeout_error(&self.cmd, self.timeout, None, None));
            }
        }

        match queue.chunks.pop_front() {
            Some(Some(chunk)) => Ok(chunk),
            // The sentinel is Python's `b''`.
            _ => Ok(Vec::new()),
        }
    }

    /// Python's `iterate_chunks()` generator, collected eagerly.
    pub fn iterate_chunks(&self) -> HydrusResult<Vec<Vec<u8>>> {
        let mut chunks = Vec::new();

        loop {
            let chunk = self.read_chunk()?;

            if chunk.is_empty() {
                return Ok(chunks);
            }

            chunks.push(chunk);
        }
    }

    fn terminate_and_reap(&self) {
        let mut process = lock(&self.process);

        terminate_and_reap_process(&mut process);
    }

    /// `close_process()`
    pub fn close_process(&self) {
        {
            let mut process = lock(&self.process);

            if !matches!(process.try_wait(), Ok(Some(_))) {
                terminate_and_reap_process(&mut process);
            }
        }

        self.closed.store(true, Ordering::Relaxed);
        self.new_chunk_desired.set();

        if let Some(reader) = lock(&self.reader).take() {
            let _ = reader.join();
        }
    }

    /// `get_return_code()`
    pub fn get_return_code(&self) -> Option<i32> {
        let mut process = lock(&self.process);

        match process.try_wait() {
            Ok(Some(status)) => status.code(),
            _ => None,
        }
    }
}

impl Drop for SubprocessContext {
    fn drop(&mut self) {
        self.close_process();
    }
}

/// `class SubprocessContextReader`
pub type SubprocessContextReader = SubprocessContext;

/// `class SubprocessContextStreamer`
pub type SubprocessContextStreamer = SubprocessContext;

#[cfg(test)]
mod tests {
    use super::*;

    fn shell_echo(text: &str) -> SubprocessCommand {
        #[cfg(windows)]
        {
            SubprocessCommand::shell(format!("echo {text}"))
        }

        #[cfg(not(windows))]
        {
            SubprocessCommand::shell(format!("echo {text}"))
        }
    }

    #[test]
    fn keyword_args_include_env_and_terminal_settings() {
        let kwargs = get_subprocess_kw_args(true, true, false);

        // The environment may be inherited (None) or rewritten; either is
        // valid. The important parts are the mode flags.
        assert!(kwargs.text);
        assert!(!kwargs.shell);
        assert_eq!(kwargs.startup_info.hide_terminal, PLATFORM_WINDOWS);
    }

    #[test]
    fn command_display() {
        assert_eq!(
            SubprocessCommand::args(["ffmpeg", "-version"]).to_string(),
            "[\"ffmpeg\", \"-version\"]"
        );
        assert_eq!(SubprocessCommand::shell("echo hi").to_string(), "echo hi");
    }

    #[test]
    fn shell_subprocess_returns_output() {
        let result = run_subprocess(
            &shell_echo("hello hydrus"),
            10,
            65536,
            false,
            true,
            true,
            false,
        )
        .unwrap();

        assert!(result.stdout_text().contains("hello hydrus"));
        assert_eq!(result.returncode, 0);
    }

    #[test]
    fn bad_return_code_message_mentions_the_code() {
        let error = report_bad_return_code(
            &SubprocessCommand::args(["false"]),
            7,
            Some(b"out"),
            Some(b"err"),
        );

        assert!(error.is_a(ErrorKind::BadReturnCode));
        assert!(error.to_string().contains("(7)"));
    }

    #[test]
    fn timeout_error_message_mentions_the_limit() {
        let error = report_timeout_error(&SubprocessCommand::shell("sleep"), 30, None, None);

        assert!(error.is_a(ErrorKind::SubprocessTimedOut));
        assert!(error.to_string().contains("over 30 seconds"));
    }

    #[test]
    fn streaming_context_reads_chunks() {
        let context =
            SubprocessContext::new(shell_echo("streamed"), 10, 65536, true, true, false).unwrap();

        let chunks = context.iterate_chunks().unwrap();

        let text = chunks
            .iter()
            .map(|chunk| String::from_utf8_lossy(chunk).to_string())
            .collect::<String>();

        assert!(text.contains("streamed"));

        context.close_process();
    }

    #[test]
    fn streaming_context_reports_return_code() {
        let context =
            SubprocessContext::new(shell_echo("done"), 10, 65536, true, true, false).unwrap();

        let _ = context.iterate_chunks();

        // The reader may finish slightly before `wait` has reaped the child.
        let started = std::time::Instant::now();

        while context.get_return_code().is_none() && started.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(context.get_return_code(), Some(0));
    }

    #[test]
    fn spawning_a_missing_program_is_an_executable_error() {
        let error = run_subprocess_raw_call(
            &SubprocessCommand::args(["definitely-not-a-real-program-hydrus"]),
            false,
            0,
            false,
            false,
            false,
            true,
            true,
            false,
        )
        .err()
        .unwrap();

        assert!(error.is_a(ErrorKind::Executable));
    }
}
