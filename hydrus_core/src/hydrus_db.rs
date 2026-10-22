//! Rust port of `hydrus/core/HydrusDB.py`.
//!
//! Python's `HydrusDB` is a base class with a controller-driven job queue and
//! dynamically registered read/write commands. This port keeps the same shape:
//!
//! * [`DBHooks`] stands in for the subclass overrides (`_create_db`,
//!   `_update_db`, module loading, command registration, ...).
//! * [`DBController`] supplies the handful of controller calls the queue needs.
//! * The queue is a `VecDeque` guarded by a `Condvar`, served by a dedicated
//!   thread the way `call_to_thread_long_running( self.main_loop )` serves it.
//! * Commands are named closures returning `Box<dyn Any + Send>`; callers
//!   downcast the job result exactly as Python callers expect the Python value.
//!
//! The standalone vacuum and large-id-query helpers at the top of the Python
//! module are free functions here too, since they operate on a throwaway
//! connection rather than the live database.

use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};
use std::thread::{JoinHandle, ThreadId};
use std::time::{Duration, Instant};

use rusqlite::Connection;

use crate::hydrus_constants::{PLATFORM_LINUX, PLATFORM_WINDOWS, SOFTWARE_VERSION};
use crate::hydrus_data;
use crate::hydrus_db_base::{
    DBBase, DBCursorTransactionWrapper, SharedConnection, TemporaryIntegerTableNameCache,
    check_has_space_for_db_transaction,
};
use crate::hydrus_db_module::DBModule;
use crate::hydrus_encryption;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_globals;
use crate::hydrus_paths;
use crate::hydrus_profiling;
use crate::hydrus_pubsub::PubSubMessage;
use crate::hydrus_static_dir;
use crate::hydrus_time;

/// `SHUTDOWN_SENTINEL` / `COMMIT_CHECK_TOKEN_SENTINEL`, as queue entries.
enum QueueItem {
    Job(Arc<DBJob>),
    Shutdown,
    CommitCheck,
}

fn db_error(context: impl std::fmt::Display, error: rusqlite::Error) -> HydrusException {
    HydrusException::new(ErrorKind::Db, format!("{context}: {error}"))
}

fn access_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::DbAccess, message)
}

fn plain_db_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::Db, message)
}

enum QueuePoll {
    Item(QueueItem),
    Timeout,
    Finished,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

/// Appends a suffix to the final component of a path (`db_path + '.prevacuum'`
/// in Python).
fn path_with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();

    name.push(suffix);

    PathBuf::from(name)
}

/// `critical_initiate_immediate_program_halt()`
///
/// Python calls `os._exit( 1 )` because `sys.exit` only raises. This port does
/// the same; it is only reached from the failed-vacuum recovery paths, where
/// continuing could corrupt the database.
pub fn critical_initiate_immediate_program_halt() -> ! {
    std::process::exit(1)
}

fn open_standalone(path: &Path) -> HydrusResult<Connection> {
    Connection::open(path)
        .map_err(|error| db_error(format!("Could not open \"{}\"", path.display()), error))
}

fn pragma_i64(connection: &Connection, pragma: &str) -> HydrusResult<i64> {
    connection
        .query_row(pragma, [], |row| row.get::<_, i64>(0))
        .map_err(|error| db_error(format!("Could not read \"{pragma}\""), error))
}

fn pragma_i64_db(db: &DBBase, pragma: &str) -> HydrusResult<i64> {
    db.query_one(pragma, &[], |row| row.get::<_, i64>(0))?
        .ok_or_else(|| plain_db_error(format!("Could not read \"{pragma}\"")))
}

/// `check_can_vacuum( db_path, stop_time = None )`
pub fn check_can_vacuum(db_path: &Path, stop_time: Option<i64>) -> HydrusResult<()> {
    let connection = open_standalone(db_path)?;

    check_can_vacuum_connection(db_path, &connection, stop_time)
}

/// The cursor form of `check_can_vacuum`, working through a [`DBBase`] so the
/// shared-connection port has the same entry point.
pub fn check_can_vacuum_cursor(
    db_path: &Path,
    db: &DBBase,
    stop_time: Option<i64>,
) -> HydrusResult<()> {
    let page_size = pragma_i64_db(db, "PRAGMA page_size;")?;
    let page_count = pragma_i64_db(db, "PRAGMA page_count;")?;
    let freelist_count = pragma_i64_db(db, "PRAGMA freelist_count;")?;

    check_can_vacuum_data(db_path, page_size, page_count, freelist_count, stop_time)
}

fn check_can_vacuum_connection(
    db_path: &Path,
    connection: &Connection,
    stop_time: Option<i64>,
) -> HydrusResult<()> {
    let page_size = pragma_i64(connection, "PRAGMA page_size;")?;
    let page_count = pragma_i64(connection, "PRAGMA page_count;")?;
    let freelist_count = pragma_i64(connection, "PRAGMA freelist_count;")?;

    check_can_vacuum_data(db_path, page_size, page_count, freelist_count, stop_time)
}

/// `check_can_vacuum_data( db_path, page_size, page_count, freelist_count, ... )`
pub fn check_can_vacuum_data(
    db_path: &Path,
    page_size: i64,
    page_count: i64,
    freelist_count: i64,
    stop_time: Option<i64>,
) -> HydrusResult<()> {
    let db_size = ((page_count - freelist_count) * page_size).max(0) as u64;

    if let Some(stop_time) = stop_time {
        let approx_vacuum_duration = get_approx_vacuum_duration(db_size);

        let time_i_will_have_to_start = stop_time - approx_vacuum_duration as i64;

        if hydrus_time::time_has_passed(Some(time_i_will_have_to_start)) {
            return Err(HydrusException::new(
                ErrorKind::Unknown,
                format!(
                    "I believe you need about {} to vacuum, but there is not enough time allotted.",
                    hydrus_time::time_delta_to_pretty_timedelta(
                        Some(approx_vacuum_duration as f64),
                        true,
                        false
                    )
                ),
            ));
        }
    }

    let db_dir = db_path.parent().unwrap_or_else(|| Path::new("."));

    check_has_space_for_db_transaction(db_dir, db_size, false)?;

    Ok(())
}

/// `check_can_vacuum_into( db_path, stop_time = None )`
pub fn check_can_vacuum_into(db_path: &Path, stop_time: Option<i64>) -> HydrusResult<()> {
    let connection = open_standalone(db_path)?;

    check_can_vacuum_into_connection(db_path, &connection, stop_time)
}

/// `check_can_vacuum_into_cursor`
pub fn check_can_vacuum_into_cursor(
    db_path: &Path,
    db: &DBBase,
    stop_time: Option<i64>,
) -> HydrusResult<()> {
    let page_size = pragma_i64_db(db, "PRAGMA page_size;")?;
    let page_count = pragma_i64_db(db, "PRAGMA page_count;")?;
    let freelist_count = pragma_i64_db(db, "PRAGMA freelist_count;")?;

    check_can_vacuum_into_data(db_path, page_size, page_count, freelist_count, stop_time)
}

fn check_can_vacuum_into_connection(
    db_path: &Path,
    connection: &Connection,
    stop_time: Option<i64>,
) -> HydrusResult<()> {
    let page_size = pragma_i64(connection, "PRAGMA page_size;")?;
    let page_count = pragma_i64(connection, "PRAGMA page_count;")?;
    let freelist_count = pragma_i64(connection, "PRAGMA freelist_count;")?;

    check_can_vacuum_into_data(db_path, page_size, page_count, freelist_count, stop_time)
}

/// `check_can_vacuum_into_data`
pub fn check_can_vacuum_into_data(
    db_path: &Path,
    page_size: i64,
    page_count: i64,
    freelist_count: i64,
    stop_time: Option<i64>,
) -> HydrusResult<()> {
    let db_size = ((page_count - freelist_count) * page_size).max(0) as u64;

    if let Some(stop_time) = stop_time {
        let approx_vacuum_duration = get_approx_vacuum_into_duration(db_size);

        let time_i_will_have_to_start = stop_time - approx_vacuum_duration as i64;

        if hydrus_time::time_has_passed(Some(time_i_will_have_to_start)) {
            return Err(HydrusException::new(
                ErrorKind::Unknown,
                format!(
                    "I believe you need about {} to vacuum, but there is not enough time allotted.",
                    hydrus_time::time_delta_to_pretty_timedelta(
                        Some(approx_vacuum_duration as f64),
                        true,
                        false
                    )
                ),
            ));
        }
    }

    let db_dir = db_path.parent().unwrap_or_else(|| Path::new("."));

    check_has_space_for_db_transaction(db_dir, db_size, true)?;

    Ok(())
}

/// `get_approx_vacuum_duration( db_size )`, in seconds.
pub fn get_approx_vacuum_duration(db_size: u64) -> u64 {
    let vacuum_estimate = (db_size as f64 * 1.2) as u64;

    let approx_vacuum_speed_bytes_per_s = 1_048_576u64;

    vacuum_estimate / approx_vacuum_speed_bytes_per_s
}

/// `get_approx_vacuum_into_duration( db_size )`, in seconds.
pub fn get_approx_vacuum_into_duration(db_size: u64) -> u64 {
    // I've seen 200MB/s on a 600MB file in memory, 30MB/s on a 4GB file not in
    // memory, and presume a real 20GB file trends a little lower.
    let vacuum_estimate = (db_size as f64 * 1.2) as u64;

    let approx_vacuum_speed_bytes_per_s = 1_048_576u64 * 10;

    vacuum_estimate / approx_vacuum_speed_bytes_per_s
}

fn random_hex_32() -> String {
    use rand::RngCore;

    let mut buffer = [0u8; 32];

    rand::thread_rng().fill_bytes(&mut buffer);

    buffer.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// `read_large_id_query_in_separate_chunks( cursor, select_statement, chunk_size )`
///
/// Returns `(chunk, num_done, num_to_do)` tuples rather than yielding, because
/// the caller usually drives this synchronously anyway.
pub fn read_large_id_query_in_separate_chunks(
    connection: &Connection,
    select_statement: &str,
    chunk_size: usize,
) -> HydrusResult<Vec<(Vec<i64>, usize, usize)>> {
    if chunk_size == 0 {
        return Err(plain_db_error("Chunk size must be greater than zero!"));
    }

    let table_name = format!("tempbigread{}", random_hex_32());

    connection
        .execute_batch(&format!(
            "CREATE TEMPORARY TABLE {table_name} ( job_id INTEGER PRIMARY KEY AUTOINCREMENT, temp_id INTEGER );"
        ))
        .map_err(|error| db_error("Could not create the temporary id table", error))?;

    // The caller's statement is expected to end in a semicolon, like Python.
    if let Err(error) = connection.execute_batch(&format!(
        "INSERT INTO {table_name} ( temp_id ) {select_statement}"
    )) {
        let _ = connection.execute_batch(&format!("DROP TABLE {table_name};"));

        return Err(db_error("Could not populate the temporary id table", error));
    }

    let num_to_do = connection.changes() as usize;

    let chunk_size_i64 = chunk_size as i64;
    let mut i = 0i64;
    let mut num_done = 0usize;
    let mut chunks = Vec::new();

    while num_done < num_to_do {
        let mut statement = connection
            .prepare(&format!(
                "SELECT temp_id FROM {table_name} WHERE job_id BETWEEN ? AND ?;"
            ))
            .map_err(|error| db_error("Could not read the temporary id table", error))?;

        let rows = statement
            .query_map(rusqlite::params![i, i + (chunk_size_i64 - 1)], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|error| db_error("Could not read the temporary id table", error))?;

        let mut chunk = Vec::new();

        for row in rows {
            chunk.push(row.map_err(|error| db_error("Could not read an id", error))?);
        }

        i += chunk_size_i64;

        if chunk.is_empty() {
            // Python would spin forever here; an empty chunk means the temp
            // table disagreed with `rowcount`, so stop rather than hang.
            break;
        }

        num_done += chunk.len();

        chunks.push((chunk, num_done, num_to_do));
    }

    connection
        .execute_batch(&format!("DROP TABLE {table_name};"))
        .map_err(|error| db_error("Could not drop the temporary id table", error))?;

    Ok(chunks)
}

/// `vacuum_db( db_path )`
pub fn vacuum_db(db_path: &Path) -> HydrusResult<()> {
    let connection = open_standalone(db_path)?;

    let journal_mode = hydrus_globals::db_journal_mode();

    // This used to be 1024 for Linux users, so coerce back to SQLite's default.
    let ideal_page_size = 4096;

    let page_size = pragma_i64(&connection, "PRAGMA page_size;")?;

    if page_size != ideal_page_size {
        connection
            .execute_batch("PRAGMA journal_mode = TRUNCATE;")
            .map_err(|error| db_error("Could not set the journal mode", error))?;

        connection
            .execute_batch(&format!("PRAGMA page_size = {ideal_page_size};"))
            .map_err(|error| db_error("Could not set the page size", error))?;
    }

    connection
        .execute_batch("PRAGMA auto_vacuum = 0;")
        .map_err(|error| db_error("Could not disable auto-vacuum", error))?;

    connection
        .execute_batch("VACUUM;")
        .map_err(|error| db_error("Could not vacuum", error))?;

    connection
        .execute_batch(&format!("PRAGMA journal_mode = {journal_mode};"))
        .map_err(|error| db_error("Could not restore the journal mode", error))?;

    Ok(())
}

/// `get_vacuum_paths( db_path )`
pub fn get_vacuum_paths(db_path: &Path) -> (PathBuf, PathBuf) {
    (
        path_with_suffix(db_path, ".prevacuum"),
        path_with_suffix(db_path, ".vacuum"),
    )
}

/// `vacuum_db_into( db_path )`
pub fn vacuum_db_into(db_path: &Path) -> HydrusResult<()> {
    if path_with_suffix(db_path, "-wal").exists() {
        return Err(HydrusException::new(
            ErrorKind::Unknown,
            format!(
                "Hey, I wanted to vacuum \"{}\", but it seems like the \"WAL\" journal file still existed after hydrus disconnected, which suggests another program is connected to the database. It may also mean your version of SQLite does not clean up after itself, in which case I still cannot be sure we are clear. Vacuuming under these conditions can cause malformation, so the job will now be abandoned. Please disconnect your external program cleanly and then try again.",
                db_path.display()
            ),
        ));
    }

    if path_with_suffix(db_path, "-journal").exists() {
        return Err(HydrusException::new(
            ErrorKind::Unknown,
            format!(
                "Hey, I wanted to vacuum \"{}\", but I see a \"TRUNCATE\" journal file even after I disconnected. I do not easily know, in TRUNCATE mode, if there are other programs connected to the database. Vacuuming under these conditions can cause malformation, so the job will now be abandoned. Please run the program in WAL journalling mode before trying again. If you have not specifically told hydrus to run in TRUNCATE mode, it probably fell back to this because WAL is not supported on your machine.",
                db_path.display()
            ),
        ));
    }

    let started = Instant::now();

    let (deletee_path, vacuum_path) = get_vacuum_paths(db_path);

    if vacuum_path.exists() {
        return Err(HydrusException::new(
            ErrorKind::Unknown,
            format!(
                "Hey, I wanted to vacuum \"{}\", but \"{}\", which I wanted to use, already existed! Did a recent vacuum attempt fail? Please delete \"{}\" yourself before you try again.",
                db_path.display(),
                vacuum_path.display(),
                vacuum_path.display()
            ),
        ));
    }

    if deletee_path.exists() {
        return Err(HydrusException::new(
            ErrorKind::Unknown,
            format!(
                "Hey, I wanted to vacuum \"{}\", but \"{}\", which I wanted to use briefly, already existed! Did a recent vacuum attempt fail? Please delete \"{}\" yourself before you try again.",
                db_path.display(),
                deletee_path.display(),
                deletee_path.display()
            ),
        ));
    }

    let original_size = std::fs::metadata(db_path)
        .map(|metadata| metadata.len())
        .map_err(|error| {
            HydrusException::new(
                ErrorKind::Unknown,
                format!(
                    "Could not determine the size of \"{}\": {error}",
                    db_path.display()
                ),
            )
        })?;

    {
        let connection = open_standalone(db_path)?;

        let vacuum_path_string = vacuum_path.to_string_lossy().to_string();

        connection
            .execute("VACUUM INTO ?;", [&vacuum_path_string])
            .map_err(|error| db_error("Could not vacuum into a new file", error))?;
    }

    if let Err(error) = std::fs::rename(db_path, &deletee_path) {
        return Err(HydrusException::new(
            ErrorKind::Unknown,
            format!(
                "While attempting to vacuum \"{}\", I could not rename it to \"{}\"! You now have a spare vacuum file in your database directory at \"{}\", which you should manually delete. ({error})",
                db_path.display(),
                deletee_path.display(),
                vacuum_path.display()
            ),
        ));
    }

    if let Err(error) = std::fs::rename(&vacuum_path, db_path) {
        if let Err(error2) = std::fs::rename(&deletee_path, db_path) {
            let error = format!("{error}");
            let error2 = format!("{error2}");

            let mut message = format!(
                "While attempting to vacuum \"{}\", I could not rename \"{}\" to \"{}\"! This is a bad situation, because now there is no database file in the desired location!",
                db_path.display(),
                vacuum_path.display(),
                db_path.display()
            );

            message += "\n\n";
            message += &format!(
                "Also, when I attempted to recover by renaming \"{}\" back to \"{}\", that also failed, even though I only just renamed it the other way! It is like your hard drive suddenly disconnected!",
                deletee_path.display(),
                db_path.display()
            );
            message += "\n\n";
            message += "Go into your db directory now and review the paths. I recommend renaming the \"prevacuum\" file back to the original name, since we do not know if this vacuum file is ok. Contact hydev if you need more help. Do not delete the \"vacuum\" file until you know things are good.";
            message += "\n\n";
            message += "The program will exit suddenly and rudely after this message is closed.";

            hydrus_data::safe_print(&error);
            hydrus_data::safe_print(&error2);

            critical_initiate_immediate_program_halt();
        }

        let mut message = format!(
            "The vacuum failed, but I have successfully rolled back to where you started. While attempting to vacuum \"{}\", I could not rename \"{}\" to \"{}\"! This was a very bad situation, but I have recovered by renaming the original, to-be-deleted \"{}\" back to \"{}\".",
            db_path.display(),
            vacuum_path.display(),
            db_path.display(),
            deletee_path.display(),
            db_path.display()
        );

        message += "\n\n";
        message += &format!(
            "You now have a spare \"{}\" file in your db dir that you should delete. You should investigate what is wrong with your database folder--could there be a permissions error, or something with free space? Why can I rename the main file but not the new vacuum? Hydev would be interested in anything you learn.",
            vacuum_path.display()
        );

        return Err(HydrusException::new(ErrorKind::Unknown, message));
    }

    if let Err(error) = std::fs::remove_file(&deletee_path) {
        hydrus_data::safe_print(format!(
            "Hey, the vacuum of \"{}\" went ok, but I could not delete the leftover file \"{}\"! Does hydrus not have delete permission on your db folder? In any case, please delete this file yourself. ({error})",
            db_path.display(),
            deletee_path.display()
        ));
    }

    let vacuum_size = std::fs::metadata(db_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);

    let time_took = started.elapsed().as_secs_f64();

    let bytes_per_sec = if time_took > 0.0 {
        (vacuum_size as f64 / time_took) as u64
    } else {
        vacuum_size
    };

    hydrus_data::safe_print(format!(
        "Vacuumed {} in {} ({}/s). It went from {} to {}",
        db_path.display(),
        hydrus_time::time_delta_to_pretty_timedelta(Some(time_took), true, false),
        hydrus_data::to_human_bytes(bytes_per_sec),
        hydrus_data::to_human_bytes(original_size),
        hydrus_data::to_human_bytes(vacuum_size)
    ));

    Ok(())
}

/// The controller calls `HydrusDB`'s job queue needs.
///
/// Python reaches the real controller through `HG.controller`; the core crate
/// does not own it, so the caller supplies this view at construction.
pub trait DBController: Send + Sync {
    /// `controller.currently_idle()`
    fn currently_idle(&self) -> bool {
        true
    }

    /// `controller.last_shutdown_was_bad()`
    fn last_shutdown_was_bad(&self) -> bool {
        false
    }

    /// `controller.blocking_safe_show_critical_message( title, message )`
    fn blocking_safe_show_critical_message(&self, title: &str, message: &str) {
        hydrus_data::debug_print(format!("{title}\n\n{message}"));
    }

    /// `controller.publish( topic, *args, **kwargs )`
    fn publish(&self, message: PubSubMessage) {
        let _ = message;
    }
}

/// A controller that never interrupts and drops published messages.
pub struct NullDBController;

impl DBController for NullDBController {}

/// A named db command. Rust cannot pass Python's `*args, **kwargs` through a
/// trait object without an encoding tax, so commands capture their arguments
/// and return a type-erased result.
pub type DbCommand = Arc<dyn Fn() -> HydrusResult<Box<dyn Any + Send>> + Send + Sync>;

/// `{ command_name: method }`
pub type DbCommands = HashMap<String, DbCommand>;

fn null_db_command() -> DbCommand {
    Arc::new(|| Ok(Box::new(()) as Box<dyn Any + Send>))
}

/// Python's subclass overrides of `HydrusDB`.
///
/// Hook implementations should not retain `DBBase` clones beyond the call that
/// receives them. A clone keeps the underlying SQLite connection alive even
/// after `HydrusDB` closes it, which prevents the WAL/shm and durable-temp files
/// from being released.
pub trait DBHooks: Send + Sync {
    /// `_create_db`
    fn create_db(
        &self,
        _db: &DBBase,
        _cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
    ) -> HydrusResult<()> {
        Err(HydrusException::new(
            ErrorKind::Unknown,
            "create_db is not implemented for this database!",
        ))
    }

    /// `_update_db( version )`. The version argument is the version being
    /// upgraded *from*, matching Python.
    fn update_db(
        &self,
        _db: &DBBase,
        _cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
        _version: i64,
    ) -> HydrusResult<()> {
        Err(HydrusException::new(
            ErrorKind::Unknown,
            "update_db is not implemented for this database!",
        ))
    }

    /// `_manage_db_error( job, e )`
    ///
    /// Python implementations put the wrapped result on the job here (the
    /// client wraps the error in a `DBException`; the server adds a traceback).
    /// If a synchronous job still has no result when this returns, the queue
    /// delivers the raw error so a caller can never hang.
    fn manage_db_error(&self, job: &DBJob, error: &HydrusException) -> HydrusResult<()> {
        hydrus_data::safe_print(format!(
            "The database job \"{}\" failed: {}",
            job.to_string(),
            hydrus_data::print_exception(error, false)
        ));

        Ok(())
    }

    /// `_init_caches`
    fn init_caches(&self, _db: &DBBase) -> HydrusResult<()> {
        Ok(())
    }

    /// `_load_modules`
    fn load_modules(&self, _db: &DBBase) -> HydrusResult<Vec<Box<dyn DBModule + Send + Sync>>> {
        Ok(Vec::new())
    }

    /// `_init_commands_to_methods`
    fn init_commands(&self, _db: &DBBase) -> HydrusResult<(DbCommands, DbCommands)> {
        Ok((
            DbCommands::new(),
            DbCommands::from([("null".to_string(), null_db_command())]),
        ))
    }

    /// The overridable `publish_status_update`; Python's subclass reads the
    /// current status off `self`, so the values are passed in here.
    fn publish_status_update(&self, _status: &str, _job_name: &str) {}

    /// `_report_overupdated_db`
    fn report_overupdated_db(&self, _version: i64) {}

    /// `_report_underupdated_db`
    fn report_underupdated_db(&self, _version: i64) {}

    /// `_display_catastrophic_error`
    fn display_catastrophic_error(&self, text: &str) {
        hydrus_data::debug_print(text);
    }
}

/// The base implementation with every hook at its Python default.
pub struct BaseDBHooks;

impl DBHooks for BaseDBHooks {}

/// Configuration that Python's subclasses set in `__init__` /
/// `_init_external_databases`.
#[derive(Clone, Debug)]
pub struct HydrusDBConfig {
    pub db_dir: PathBuf,
    pub db_name: String,
    /// `( schema_name, filename )`; `main` is added automatically and must not
    /// appear here.
    pub external_databases: Vec<(String, String)>,
    /// `_get_possible_additional_db_filenames`
    pub additional_db_filenames: Vec<String>,
    pub durable_temp_db_filename: Option<String>,
    pub ssl_cert_filename: Option<String>,
    pub ssl_key_filename: Option<String>,
    /// `READ_WRITE_ACTIONS`
    pub read_write_actions: Vec<String>,
    /// `UPDATE_WAIT`, in seconds.
    pub update_wait_seconds: u64,
}

impl HydrusDBConfig {
    pub fn new(db_dir: impl Into<PathBuf>, db_name: impl Into<String>) -> Self {
        Self {
            db_dir: db_dir.into(),
            db_name: db_name.into(),
            external_databases: Vec::new(),
            additional_db_filenames: Vec::new(),
            durable_temp_db_filename: None,
            ssl_cert_filename: None,
            ssl_key_filename: None,
            read_write_actions: Vec::new(),
            update_wait_seconds: 2,
        }
    }
}

/// Python's `JobDatabase`, carrying an action name instead of a callable.
pub struct DBJob {
    job_type: String,
    synchronous: bool,
    action: String,
    result: Mutex<Option<HydrusResult<Box<dyn Any + Send>>>>,
    result_ready: Condvar,
}

impl DBJob {
    pub fn new(job_type: impl Into<String>, synchronous: bool, action: impl Into<String>) -> Self {
        Self {
            job_type: job_type.into(),
            synchronous,
            action: action.into(),
            result: Mutex::new(None),
            result_ready: Condvar::new(),
        }
    }

    pub fn get_type(&self) -> &str {
        &self.job_type
    }

    pub fn is_synchronous(&self) -> bool {
        self.synchronous
    }

    /// `get_callable_tuple`'s action component.
    pub fn action(&self) -> &str {
        &self.action
    }

    /// `put_result`
    pub fn put_result(&self, value: HydrusResult<Box<dyn Any + Send>>) {
        {
            *lock(&self.result) = Some(value);
        }

        self.result_ready.notify_all();
    }

    /// Whether a result has been delivered, so a hook can safely be checked
    /// before the queue delivers a raw error fallback.
    pub fn has_result(&self) -> bool {
        lock(&self.result).is_some()
    }

    /// `get_result`
    pub fn get_result(&self) -> HydrusResult<Box<dyn Any + Send>> {
        let mut result = lock(&self.result);

        while result.is_none() {
            result = self
                .result_ready
                .wait(result)
                .unwrap_or_else(|error| error.into_inner());
        }

        result.take().unwrap()
    }
}

impl std::fmt::Display for DBJob {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} {}", self.job_type, self.action)
    }
}

struct DBState {
    connection: Option<SharedConnection>,
    wrapper: Option<DBCursorTransactionWrapper>,
    modules: Vec<Box<dyn DBModule + Send + Sync>>,
    read_commands: DbCommands,
    write_commands: DbCommands,
    is_connected: bool,
    current_status: String,
    current_job_name: String,
    currently_doing_job: bool,
    ready_to_serve_requests: bool,
    could_not_initialise: bool,
}

impl DBState {
    fn new() -> Self {
        Self {
            connection: None,
            wrapper: None,
            modules: Vec::new(),
            read_commands: DbCommands::new(),
            write_commands: DbCommands::new(),
            is_connected: false,
            current_status: String::new(),
            current_job_name: String::new(),
            currently_doing_job: false,
            ready_to_serve_requests: false,
            could_not_initialise: false,
        }
    }
}

/// Python's `HydrusDB`.
pub struct HydrusDB {
    config: HydrusDBConfig,
    hooks: Arc<dyn DBHooks>,
    controller: Arc<dyn DBController>,

    db_filenames: HashMap<String, String>,
    durable_temp_db_filename: String,
    ssl_cert_path: PathBuf,
    ssl_key_path: PathBuf,

    state: Mutex<DBState>,
    startup: Condvar,

    queue: Mutex<VecDeque<QueueItem>>,
    queue_changed: Condvar,

    idle: Mutex<bool>,
    idle_changed: Condvar,

    thread: Mutex<Option<JoinHandle<()>>>,
    thread_id: Mutex<Option<ThreadId>>,

    is_first_start: AtomicBool,
    is_db_updated: AtomicBool,
    local_shutdown: AtomicBool,
    pause_and_disconnect: AtomicBool,
    loop_finished: AtomicBool,
    we_have_connected_at_least_once: AtomicBool,
}

impl HydrusDB {
    /// Construct, initialise, and start a database.
    ///
    /// This mirrors Python's `__init__`: the create/repair/update flow runs on
    /// the calling thread, then the job queue thread takes over.
    pub fn new(
        config: HydrusDBConfig,
        hooks: Arc<dyn DBHooks>,
        controller: Arc<dyn DBController>,
    ) -> HydrusResult<Arc<Self>> {
        let main_filename = if config.db_name.ends_with(".db") {
            config.db_name.clone()
        } else {
            format!("{}.db", config.db_name)
        };

        let mut db_filenames = HashMap::new();

        db_filenames.insert("main".to_string(), main_filename);

        for (name, filename) in &config.external_databases {
            if name == "main" {
                return Err(access_error(
                    "\"main\" is reserved and cannot be used as an external database name.",
                ));
            }

            if !is_safe_identifier(name) {
                return Err(access_error(format!(
                    "\"{name}\" is not a valid external database name!"
                )));
            }

            db_filenames.insert(name.clone(), filename.clone());
        }

        let durable_temp_db_filename = config
            .durable_temp_db_filename
            .clone()
            .unwrap_or_else(|| format!("{}.temp.db", config.db_name));

        let ssl_cert_filename = config
            .ssl_cert_filename
            .clone()
            .unwrap_or_else(|| format!("{}.crt", config.db_name));

        let ssl_key_filename = config
            .ssl_key_filename
            .clone()
            .unwrap_or_else(|| format!("{}.key", config.db_name));

        let ssl_cert_path = config.db_dir.join(ssl_cert_filename);
        let ssl_key_path = config.db_dir.join(ssl_key_filename);

        let durable_temp_db_path = config.db_dir.join(&durable_temp_db_filename);

        if durable_temp_db_path.exists() {
            hydrus_paths::delete_path(&durable_temp_db_path);

            for suffix in ["-wal", "-shm"] {
                let sidecar = path_with_suffix(&durable_temp_db_path, suffix);

                if sidecar.exists() {
                    hydrus_paths::delete_path(&sidecar);
                }
            }

            hydrus_data::safe_print(
                "Found and deleted the durable temporary database on boot. The last exit was probably not clean.",
            );
        }

        let db = Arc::new(Self {
            config,
            hooks,
            controller,
            db_filenames,
            durable_temp_db_filename,
            ssl_cert_path,
            ssl_key_path,
            state: Mutex::new(DBState::new()),
            startup: Condvar::new(),
            queue: Mutex::new(VecDeque::new()),
            queue_changed: Condvar::new(),
            idle: Mutex::new(true),
            idle_changed: Condvar::new(),
            thread: Mutex::new(None),
            thread_id: Mutex::new(None),
            is_first_start: AtomicBool::new(false),
            is_db_updated: AtomicBool::new(false),
            local_shutdown: AtomicBool::new(false),
            pause_and_disconnect: AtomicBool::new(false),
            loop_finished: AtomicBool::new(false),
            we_have_connected_at_least_once: AtomicBool::new(false),
        });

        db.initialise()?;

        Ok(db)
    }

    /// Convenience wrapper so callers do not have to coerce hook/controller
    /// values to `Arc<dyn ...>` themselves.
    pub fn from_parts<H, C>(
        config: HydrusDBConfig,
        hooks: H,
        controller: C,
    ) -> HydrusResult<Arc<Self>>
    where
        H: DBHooks + 'static,
        C: DBController + 'static,
    {
        Self::new(config, Arc::new(hooks), Arc::new(controller))
    }

    fn initialise(self: &Arc<Self>) -> HydrusResult<()> {
        let main_db_path = self.main_db_path();

        if main_db_path.exists() {
            // Open and close to clean up in case the last session did not close
            // well.
            self.init_db()?;
            self.close_db_connection()?;
        }

        let (space_wanted, free_space) =
            self.get_safe_transaction_disk_space_and_current_free_space();

        if let Some(free_space) = free_space
            && free_space < space_wanted
        {
            return Err(access_error(format!(
                "Sorry, it looks like the database drive partition has less than {} free space. It needs this for database transactions, so please free up some space.",
                hydrus_data::to_human_bytes(space_wanted)
            )));
        }

        self.init_db()?;

        let mut version = self.get_version()?;

        if version > SOFTWARE_VERSION as i64 {
            self.hooks.report_overupdated_db(version);
        }

        if version < SOFTWARE_VERSION as i64 - 50 {
            return Err(HydrusException::new(
                ErrorKind::DbVersion,
                format!(
                    "Your current database version of hydrus {} is too old for this software version {} to update. Please try updating with version {} or earlier first.",
                    version,
                    SOFTWARE_VERSION,
                    version + 45
                ),
            ));
        }

        // ( bitrot_db_name, latest_affected_version, safe_update_version, reason )
        let bitrot_rows = [("client", 551i64, 558i64, "millisecond timestamp conversion")];

        for (bitrot_db_name, latest_affected_version, safe_update_version, reason) in bitrot_rows {
            if self.config.db_name == bitrot_db_name && version <= latest_affected_version {
                return Err(HydrusException::new(
                    ErrorKind::DbVersion,
                    format!(
                        "Sorry, due to a bitrot issue ({reason}), you cannot update to this software version (v{SOFTWARE_VERSION}) if your database is on v{latest_affected_version} or earlier (you are on v{version}). Please download and update to v{safe_update_version} first!"
                    ),
                ));
            }
        }

        if version < SOFTWARE_VERSION as i64 - 15 {
            self.hooks.report_underupdated_db(version);
        }

        self.repair_db(version)?;

        while version < SOFTWARE_VERSION as i64 {
            if self.config.update_wait_seconds > 0 {
                std::thread::sleep(Duration::from_secs(self.config.update_wait_seconds));
            }

            if let Err(error) = self.with_taken_wrapper(|wrapper| wrapper.begin_immediate()) {
                return Err(access_error(error.to_string()));
            }

            let db = self.db_base()?;

            let update_result =
                self.with_taken_wrapper(|wrapper| self.hooks.update_db(&db, wrapper, version));

            match update_result {
                Ok(()) => {
                    self.with_taken_wrapper(|wrapper| wrapper.commit())?;

                    self.is_db_updated.store(true, Ordering::Relaxed);
                }
                Err(error) => {
                    let error = HydrusException::new(
                        ErrorKind::Unknown,
                        format!(
                            "Updating the {} db to version {} caused this error:\n{}",
                            self.config.db_name,
                            version + 1,
                            hydrus_data::print_exception(&error, false)
                        ),
                    );

                    if let Err(rollback_error) =
                        self.with_taken_wrapper(|wrapper| wrapper.rollback())
                    {
                        hydrus_data::safe_print(
                            "When the update failed, attempting to rollback the database failed.",
                        );
                        hydrus_data::safe_print(hydrus_data::print_exception(
                            &rollback_error,
                            false,
                        ));
                    }

                    return Err(error);
                }
            }

            version = self.get_version()?;
        }

        self.close_db_connection()?;

        let thread_db = Arc::downgrade(self);

        let handle = std::thread::Builder::new()
            .name(format!("hydrus db {}", self.config.db_name))
            .spawn(move || Self::thread_entry(thread_db))
            .map_err(|error| {
                HydrusException::new(
                    ErrorKind::Unknown,
                    format!("Could not start the database thread: {error}"),
                )
            })?;

        *lock(&self.thread) = Some(handle);

        self.wait_until_ready()
    }

    fn main_db_path(&self) -> PathBuf {
        // `main` is always inserted by `new`.
        self.config
            .db_dir
            .join(self.db_filenames.get("main").expect("main db filename"))
    }

    fn external_db_paths(&self) -> Vec<(String, PathBuf)> {
        let mut rows = self
            .db_filenames
            .iter()
            .filter(|(name, _)| name.as_str() != "main")
            .map(|(name, filename)| (name.clone(), self.config.db_dir.join(filename)))
            .collect::<Vec<_>>();

        rows.sort_by(|a, b| a.1.cmp(&b.1));

        rows
    }

    fn get_version(&self) -> HydrusResult<i64> {
        let db = self.db_base()?;

        let version = db
            .query_one("SELECT version FROM version;", &[], |row| {
                row.get::<_, i64>(0)
            })?
            .ok_or_else(|| plain_db_error("The database version table was empty!"))?;

        Ok(version)
    }

    /// `_init_db`
    fn init_db(&self) -> HydrusResult<()> {
        let main_db_path = self.main_db_path();
        let main_database_is_missing = !main_db_path.exists();

        let external_db_paths = self
            .external_db_paths()
            .into_iter()
            .map(|(_, path)| path)
            .collect::<Vec<_>>();

        let mut all_db_paths = vec![main_db_path.clone()];
        all_db_paths.extend(external_db_paths.iter().cloned());

        let missing_db_paths = all_db_paths
            .iter()
            .filter(|path| !path.exists())
            .cloned()
            .collect::<Vec<_>>();

        if self.we_have_connected_at_least_once.load(Ordering::Relaxed)
            && !missing_db_paths.is_empty()
        {
            let missing_paths_summary = missing_db_paths
                .iter()
                .map(|path| format!("\"{}\"", path.display()))
                .collect::<Vec<_>>()
                .join(", ");

            let message = format!(
                "Holy hell, it looks like the database files {missing_paths_summary} disappeared while the program was working! Obviously something very bad has happened, and the program will now immediately quit. You will have to investigate this. hydev can help you figure it out."
            );

            self.controller
                .blocking_safe_show_critical_message("recovering from failed vacuum!", &message);

            critical_initiate_immediate_program_halt();
        }

        let none_of_the_database_files_exist = missing_db_paths.len() == all_db_paths.len();

        let we_have_damage_from_missing_files =
            !missing_db_paths.is_empty() && !none_of_the_database_files_exist;

        if we_have_damage_from_missing_files {
            for missing_db_path in &missing_db_paths {
                let (deletee_path, vacuum_path) = get_vacuum_paths(missing_db_path);

                if deletee_path.exists() {
                    let mut message = format!(
                        "The database file \"{}\" was missing, but I saw a file called \"{}\". It appears you have just had a failed vacuum.",
                        missing_db_path.display(),
                        deletee_path.display()
                    );

                    message += "\n\n";
                    message += "If you ok this message, hydrus will attempt to move this file back to where it belongs, restoring you to where you were before the failed vacuum. This is the best that hydev can do in this situation and it is almost certainly the correct thing to do.";
                    message += "\n\n";
                    message += "If you do know better and want to check your hard drive for other damage before attempting repairs, kill the hydrus process now.";

                    if vacuum_path.exists() {
                        message += "\n\n";
                        message += &format!(
                            "There is also a file called \"{}\". This is the file that should have become your new vacuumed database file. You should not delete this file yet, since we may need it for recovery purposes if \"{}\" is damaged, but it is the product of a failed vacuum operation and cannot be trusted, so you should plan to delete it if everything else goes well.",
                            vacuum_path.display(),
                            deletee_path.display()
                        );
                    }

                    self.controller.blocking_safe_show_critical_message(
                        "recovering from failed vacuum!",
                        &message,
                    );

                    if let Err(error) = std::fs::rename(&deletee_path, missing_db_path) {
                        hydrus_data::safe_print(format!(
                            "Could not rename \"{}\" to \"{}\": {error}",
                            deletee_path.display(),
                            missing_db_path.display()
                        ));

                        let message = "I am sorry, the recovery attempt failed because I could not rename the file. It looks like your hard drive is damaged or somehow read-only. Hydrus will now quit; please check out what could be wrong with your drive, and ask hydev for help if you need it!";

                        self.controller
                            .blocking_safe_show_critical_message("REPAIR FAILURE", message);

                        critical_initiate_immediate_program_halt();
                    }

                    return self.init_db();
                }
            }

            let missing_external_db_paths = external_db_paths
                .iter()
                .filter(|path| missing_db_paths.contains(*path))
                .cloned()
                .collect::<Vec<_>>();

            let existing_external_paths_summary = external_db_paths
                .iter()
                .filter(|path| !missing_external_db_paths.contains(*path))
                .map(|path| format!("\"{}\"", path.display()))
                .collect::<Vec<_>>()
                .join(", ");

            let missing_external_paths_summary = missing_external_db_paths
                .iter()
                .map(|path| format!("\"{}\"", path.display()))
                .collect::<Vec<_>>()
                .join(", ");

            if main_database_is_missing {
                let message = if !missing_external_db_paths.is_empty() {
                    format!(
                        "Although the expected external files, {existing_external_paths_summary} do exist, {missing_external_paths_summary} and, most importantly, the main database file, \"{}\", do not!\n\nThe missing main database file makes for an invalid database, and the program will now quit. Please contact hydrus_dev if you do not know how this happened or need help recovering from hard drive failure.",
                        main_db_path.display()
                    )
                } else {
                    format!(
                        "Although all the expected external files, {existing_external_paths_summary} do exist, the main database file, \"{}\", does not!\n\nThis makes for an invalid database, and the program will now quit. Please contact hydrus_dev if you do not know how this happened or need help recovering from hard drive failure.",
                        main_db_path.display()
                    )
                };

                return Err(access_error(message));
            } else if !missing_external_db_paths.is_empty() {
                let message = format!(
                    "While the main database file, \"{}\", exists, the external files {missing_external_paths_summary} do not!\n\nIf this is a surprise to you, you have probably had a hard drive failure. You must close this process immediately and diagnose what has happened. Check the 'Recovery->Help my db is broke' document in the help for additional reading.\n\nIf this is not a surprise, then you may continue if you wish, and hydrus will do its best to reconstruct the missing files. You will see more error prompts.",
                    main_db_path.display()
                );

                self.controller
                    .blocking_safe_show_critical_message("missing database file!", &message);
            }
        }

        self.init_db_connection()?;

        if !self
            .we_have_connected_at_least_once
            .swap(true, Ordering::Relaxed)
        {
            if none_of_the_database_files_exist {
                self.is_first_start.store(true, Ordering::Relaxed);

                let db = self.db_base()?;

                self.with_taken_wrapper(|wrapper| self.hooks.create_db(&db, wrapper))?;

                self.update_db_folder_with_supplementary_files()?;

                self.with_taken_wrapper(|wrapper| wrapper.commit_and_begin())?;
            } else {
                let db = self.db_base()?;

                let result = db.query_one(
                    "SELECT 1 FROM sqlite_master WHERE type = ? AND name = ?;",
                    &[&"table", &"version"],
                    |row| row.get::<_, i64>(0),
                )?;

                if result.is_none() {
                    let mut message = format!(
                        "The \"version\" table in your \"{}\" database was missing. I cannot recover from this automatically.",
                        main_db_path.display()
                    );

                    message += "\n\n";
                    message += "If you have used this database many times before, then you have probably had a hard drive failure. Hydrus will now close. Check the 'Recovery->Help my db is broke' document in the help.";
                    message += "\n\n";
                    message += "If you recently tried to start hydrus for the first time but got an error and are now trying again, your database files failed to initialise that first time and are now in the way. Go to your database folder (probably install_dir/db) and look at the .db files. Are they very small? Delete them and any .db-shm or .db-wal files with them and then try booting the program again. If you get the same problem over and over, you probably have a hard drive permission problem where hydrus can create the database files but not write to them. If the issue seems complicated, hydev can help you figure it all out.";

                    return Err(access_error(format!(
                        "missing critical database table!\n{message}"
                    )));
                }
            }
        }

        Ok(())
    }

    /// `_init_db_connection`
    fn init_db_connection(&self) -> HydrusResult<()> {
        self.close_db_connection()?;

        let db_path = self.main_db_path();

        if db_path.exists() && !hydrus_paths::file_is_writeable(&db_path) {
            return Err(access_error(format!(
                "\"{}\" seems to be read-only!",
                db_path.display()
            )));
        }

        let connection = Connection::open(&db_path).map_err(|error| {
            access_error(format!(
                "Could not connect to database! If the answer is not obvious to you, please let hydrus dev know. Error follows:\n\n{error}"
            ))
        })?;

        let shared = Arc::new(Mutex::new(connection));
        let db = DBBase::new(shared.clone());

        if hydrus_globals::no_db_temp_files() {
            db.execute("PRAGMA temp_store = 2;", &[])?;
        }

        self.attach_external_databases(&db)?;

        let modules = self.hooks.load_modules(&db)?;
        let (read_commands, write_commands) = self.hooks.init_commands(&db)?;

        db.execute("ATTACH \":memory:\" AS mem;", &[])?;

        TemporaryIntegerTableNameCache::instance().clear();

        let db_names = db.get_attached_database_names(false)?;

        let cache_size = hydrus_globals::db_cache_size() * 1024;
        let journal_mode = hydrus_globals::db_journal_mode();
        let synchronous = hydrus_globals::db_synchronous();

        for db_name in &db_names {
            if db_name == "mem" || db_name == "temp" {
                continue;
            }

            if !is_safe_identifier(db_name) {
                return Err(access_error(format!(
                    "\"{db_name}\" is not a valid database name!"
                )));
            }

            // MB -> KB
            db.execute_batch(&format!("PRAGMA {db_name}.cache_size = -{cache_size};"))?;

            db.execute_batch(&format!("PRAGMA {db_name}.journal_mode = {journal_mode};"))?;

            if journal_mode == "PERSIST" || journal_mode == "WAL" {
                db.execute_batch(&format!(
                    "PRAGMA {db_name}.journal_size_limit = {};",
                    crate::hydrus_db_base::JOURNAL_SIZE_LIMIT
                ))?;
            }

            db.execute_batch(&format!("PRAGMA {db_name}.synchronous = {synchronous};"))?;

            if let Err(error) = db.query_one(
                &format!("SELECT * FROM {db_name}.sqlite_master;"),
                &[],
                |_| Ok(()),
            ) {
                let message = format!(
                    "The database seemed valid, but hydrus failed to read basic data from it. You may need to run the program in a different journal mode using --db_journal_mode. Full error information:\n\n{error}"
                );

                hydrus_data::debug_print(&message);

                return Err(access_error(message));
            }
        }

        let mut wrapper = DBCursorTransactionWrapper::new(
            shared.clone(),
            hydrus_globals::db_transaction_commit_period() as i64,
        );

        if let Err(error) = wrapper.begin_immediate() {
            if error.to_string().contains("locked") {
                return Err(access_error(
                    "Database appeared to be locked. Please ensure there is not another client already running on this database, and then try restarting the client.",
                ));
            }

            return Err(access_error(error.to_string()));
        }

        {
            let mut state = lock(&self.state);

            state.connection = Some(shared);
            state.wrapper = Some(wrapper);
            state.modules = modules;
            state.read_commands = read_commands;
            state.write_commands = write_commands;
            state.is_connected = true;
        }

        Ok(())
    }

    fn attach_external_databases(&self, db: &DBBase) -> HydrusResult<()> {
        for (name, filename) in &self.config.external_databases {
            if name == "main" {
                continue;
            }

            let db_path = self.config.db_dir.join(filename);

            if db_path.exists() && !hydrus_paths::file_is_writeable(&db_path) {
                return Err(access_error(format!(
                    "\"{}\" seems to be read-only!",
                    db_path.display()
                )));
            }

            let path_string = db_path.to_string_lossy().to_string();

            db.execute(&format!("ATTACH ? AS {name};"), &[&path_string])?;
        }

        let temp_path = self.config.db_dir.join(&self.durable_temp_db_filename);
        let path_string = temp_path.to_string_lossy().to_string();

        db.execute("ATTACH ? AS durable_temp;", &[&path_string])?;

        Ok(())
    }

    /// `_close_db_connection`
    fn close_db_connection(&self) -> HydrusResult<()> {
        TemporaryIntegerTableNameCache::instance().clear();

        let mut wrapper = {
            let mut state = lock(&self.state);

            state.connection.take();
            state.wrapper.take()
        };

        let mut result = Ok(());

        if let Some(wrapper) = wrapper.as_mut()
            && wrapper.in_transaction()
            && let Err(error) = wrapper.commit()
        {
            result = Err(error);
        }

        {
            let mut state = lock(&self.state);

            state.modules.clear();
            state.read_commands.clear();
            state.write_commands.clear();
            state.is_connected = false;
            state.wrapper = None;
            state.connection = None;
        }

        result
    }

    /// `_load_modules` has already happened; this runs module repairs.
    fn repair_db(&self, version: i64) -> HydrusResult<()> {
        let modules = {
            let mut state = lock(&self.state);

            std::mem::take(&mut state.modules)
        };

        let last_shutdown_was_bad = self.controller.last_shutdown_was_bad();

        let mut wrapper = {
            let mut state = lock(&self.state);

            state.wrapper.take()
        }
        .ok_or_else(|| access_error("The database is not connected!"))?;

        let mut result = Ok(());

        for module in &modules {
            if let Err(error) = module.repair(version, &mut wrapper) {
                result = Err(error);

                break;
            }
        }

        if result.is_ok() && last_shutdown_was_bad {
            for module in &modules {
                module.do_last_shutdown_was_bad_work();
            }
        }

        {
            let mut state = lock(&self.state);

            state.modules = modules;
            state.wrapper = Some(wrapper);
        }

        result
    }

    /// `_update_db_folder_with_supplementary_files`
    fn update_db_folder_with_supplementary_files(&self) -> HydrusResult<()> {
        let sqlite_exe_path = if PLATFORM_WINDOWS {
            Some(
                hydrus_static_dir::INSTALL_STATIC_DIR
                    .join("build_files")
                    .join("windows")
                    .join("sqlite3.exe"),
            )
        } else if PLATFORM_LINUX {
            Some(
                hydrus_static_dir::INSTALL_STATIC_DIR
                    .join("build_files")
                    .join("linux")
                    .join("sqlite3"),
            )
        } else {
            None
        };

        if let Some(sqlite_exe_path) = sqlite_exe_path
            && sqlite_exe_path.exists()
            && sqlite_exe_path.is_file()
        {
            let dest_path = self
                .config
                .db_dir
                .join(sqlite_exe_path.file_name().unwrap_or_default());

            hydrus_paths::mirror_file(&sqlite_exe_path, &dest_path)?;
        }

        Ok(())
    }

    fn wait_until_ready(&self) -> HydrusResult<()> {
        let mut state = lock(&self.state);

        while !state.ready_to_serve_requests && !state.could_not_initialise {
            state = self
                .startup
                .wait(state)
                .unwrap_or_else(|error| error.into_inner());
        }

        if state.could_not_initialise {
            return Err(HydrusException::new(
                ErrorKind::Unknown,
                "Could not initialise the db! Error written to the log!",
            ));
        }

        Ok(())
    }

    fn set_ready_to_serve_requests(&self) {
        {
            let mut state = lock(&self.state);

            state.ready_to_serve_requests = true;
        }

        self.startup.notify_all();
    }

    fn set_could_not_initialise(&self) {
        {
            let mut state = lock(&self.state);

            state.could_not_initialise = true;
        }

        self.startup.notify_all();
    }

    /// The database thread's entry point.
    ///
    /// The thread only holds a `Weak` reference between jobs. That lets the
    /// last external `Arc` run `Drop` (and shut the loop down) instead of the
    /// thread keeping the database alive forever.
    fn thread_entry(weak: Weak<HydrusDB>) {
        let Some(db) = weak.upgrade() else {
            return;
        };

        *lock(&db.thread_id) = Some(std::thread::current().id());

        if let Err(error) = db.initialise_thread() {
            db.hooks
                .display_catastrophic_error(&hydrus_data::print_exception(&error, false));

            db.set_could_not_initialise();

            return;
        }

        db.set_ready_to_serve_requests();

        drop(db);

        loop {
            let Some(db) = weak.upgrade() else {
                return;
            };

            match db.poll_queue(Duration::from_millis(100)) {
                QueuePoll::Item(item) => {
                    db.handle_item(item);
                    db.finish_item();

                    drop(db);
                }
                QueuePoll::Timeout => {
                    drop(db);

                    if weak.strong_count() == 0 {
                        // The last owner went away and its `Drop` has already
                        // cleaned the database up.
                        return;
                    }
                }
                QueuePoll::Finished => {
                    db.finish_loop();

                    return;
                }
            }
        }
    }

    fn handle_item(&self, item: QueueItem) {
        match item {
            QueueItem::Shutdown => {
                // Jobs already queued are still finished off.
            }
            QueueItem::CommitCheck => {
                if let Err(error) = self.commit_if_needed() {
                    self.hooks
                        .display_catastrophic_error(&hydrus_data::print_exception(&error, false));
                }
            }
            QueueItem::Job(job) => {
                {
                    let mut state = lock(&self.state);

                    state.currently_doing_job = true;
                    state.current_job_name = job.to_string();
                }

                self.publish_status_update();

                if hydrus_globals::db_report_mode() {
                    hydrus_data::safe_print(format!("Running db job: {}", job.to_string()));
                }

                if hydrus_profiling::is_profile_mode("db") {
                    let summary = format!("Profiling db job: {}", job.to_string());

                    hydrus_profiling::profile(
                        &summary,
                        || self.process_job(&job),
                        hydrus_globals::db_profile_min_job_time_ms(),
                        false,
                    );
                } else {
                    self.process_job(&job);
                }

                {
                    let mut state = lock(&self.state);

                    state.current_job_name.clear();
                    state.currently_doing_job = false;
                }
            }
        }
    }

    fn finish_item(&self) {
        self.set_current_status("");

        if self.queue_empty() {
            self.set_idle();
        }

        if self.pause_and_disconnect.load(Ordering::Relaxed) {
            self.pause_loop();
        }
    }

    /// A thread-safe port of the `main_loop` tail.
    fn finish_loop(&self) {
        let _ = self.close_db_connection();

        let temp_path = self.config.db_dir.join(&self.durable_temp_db_filename);

        hydrus_paths::delete_path(&temp_path);

        // Mark the loop finished and drain under the queue lock, so a job
        // cannot slip in after the drain.
        {
            let mut queue = lock(&self.queue);

            self.loop_finished.store(true, Ordering::Relaxed);

            for item in queue.drain(..) {
                if let QueueItem::Job(job) = item {
                    job.put_result(Err(HydrusException::new(
                        ErrorKind::Shutdown,
                        "The database is shutting down!",
                    )));
                }
            }
        }

        self.set_idle();
    }

    fn initialise_thread(&self) -> HydrusResult<()> {
        self.init_db_connection()?;

        let db = self.db_base()?;

        self.hooks.init_caches(&db)?;

        Ok(())
    }

    fn poll_queue(&self, timeout: Duration) -> QueuePoll {
        let mut queue = lock(&self.queue);

        loop {
            if let Some(item) = queue.pop_front() {
                return QueuePoll::Item(item);
            }

            if self.local_shutdown.load(Ordering::Relaxed) || hydrus_globals::model_shutdown() {
                return QueuePoll::Finished;
            }

            let (next_queue, wait_result) = self
                .queue_changed
                .wait_timeout(queue, timeout)
                .unwrap_or_else(|error| error.into_inner());

            queue = next_queue;

            if wait_result.timed_out() {
                return QueuePoll::Timeout;
            }
        }
    }

    fn pause_loop(&self) {
        if let Err(error) = self.close_db_connection() {
            self.hooks
                .display_catastrophic_error(&hydrus_data::print_exception(&error, false));
        }

        self.set_current_status("db locked");

        while self.pause_and_disconnect.load(Ordering::Relaxed)
            && !self.local_shutdown.load(Ordering::Relaxed)
            && !hydrus_globals::model_shutdown()
        {
            std::thread::sleep(Duration::from_secs(1));
        }

        if !self.local_shutdown.load(Ordering::Relaxed) && !hydrus_globals::model_shutdown() {
            if let Err(error) = self.init_db_connection() {
                self.hooks
                    .display_catastrophic_error(&hydrus_data::print_exception(&error, false));

                self.set_could_not_initialise();
            }

            self.set_current_status("");
        }
    }

    /// `_process_job`
    fn process_job(&self, job: &DBJob) {
        let job_type = job.get_type().to_string();
        let action = job.action().to_string();
        let is_write = job_type == "read_write" || job_type == "write";

        self.set_current_status(if is_write { "db writing" } else { "db reading" });

        let idle_at_job_start = self.controller.currently_idle();
        let time_job_started = Instant::now();

        let result = (|| -> HydrusResult<Box<dyn Any + Send>> {
            if is_write {
                self.with_taken_wrapper(|wrapper| {
                    wrapper.notify_write_occuring();

                    Ok(())
                })?;
            }

            let command = self.command_for(&job_type, &action)?;

            command()
        })();

        let idle_at_job_end = self.controller.currently_idle();

        if (!idle_at_job_start || !idle_at_job_end)
            && time_job_started.elapsed().as_secs_f64() > 15.0
        {
            hydrus_data::safe_print(format!(
                "The database job \"{}\" took {}.",
                job.to_string(),
                hydrus_time::time_delta_to_pretty_timedelta(
                    Some(time_job_started.elapsed().as_secs_f64()),
                    true,
                    false
                )
            ));
        }

        match result {
            Ok(value) => {
                if job.is_synchronous() {
                    job.put_result(Ok(value));
                }

                if let Err(error) = self.finish_successful_job() {
                    self.hooks
                        .display_catastrophic_error(&hydrus_data::print_exception(&error, false));
                }
            }
            Err(error) => {
                if let Err(manage_error) = self.hooks.manage_db_error(job, &error) {
                    hydrus_data::safe_print(hydrus_data::print_exception(&manage_error, false));
                }

                // Python's client/server `_manage_db_error` delivers the
                // wrapped result. A hook that does not do so still must not
                // leave a synchronous caller blocked forever.
                if job.is_synchronous() && !job.has_result() {
                    job.put_result(Err(error.clone()));
                }

                if let Err(rollback_error) = self.with_taken_wrapper(|wrapper| wrapper.rollback()) {
                    hydrus_data::safe_print(
                        "When the transaction failed, attempting to rollback the database failed. Please restart the client as soon as is convenient.",
                    );

                    let _ = self.close_db_connection();
                    let _ = self.init_db_connection();

                    hydrus_data::safe_print(hydrus_data::print_exception(&rollback_error, false));
                }
            }
        }

        self.clean_after_job_work();

        self.set_current_status("");
    }

    fn finish_successful_job(&self) -> HydrusResult<()> {
        let time_to_commit = self.with_taken_wrapper(|wrapper| {
            wrapper.save()?;

            Ok(wrapper.time_to_commit())
        })?;

        if time_to_commit {
            self.set_current_status("db committing");

            self.with_taken_wrapper(|wrapper| wrapper.commit_and_begin())?;
        }

        self.do_after_job_work()?;

        Ok(())
    }

    fn commit_if_needed(&self) -> HydrusResult<()> {
        let time_to_commit = self.with_taken_wrapper(|wrapper| Ok(wrapper.time_to_commit()))?;

        if time_to_commit {
            self.set_current_status("db committing");

            self.with_taken_wrapper(|wrapper| wrapper.commit_and_begin())?;
        }

        self.set_current_status("");

        Ok(())
    }

    fn command_for(&self, job_type: &str, action: &str) -> HydrusResult<DbCommand> {
        let state = lock(&self.state);

        let commands = if job_type == "write" {
            &state.write_commands
        } else {
            &state.read_commands
        };

        commands.get(action).cloned().ok_or_else(|| {
            HydrusException::new(
                ErrorKind::Unknown,
                format!(
                    "db received an unknown {} command: {action}",
                    if job_type == "write" { "write" } else { "read" }
                ),
            )
        })
    }

    fn clean_after_job_work(&self) {
        if let Ok(mut wrapper) = self.take_wrapper() {
            wrapper.clean_pub_subs();

            lock(&self.state).wrapper = Some(wrapper);
        }
    }

    fn do_after_job_work(&self) -> HydrusResult<()> {
        let messages = self.with_taken_wrapper(|wrapper| Ok(wrapper.do_pub_subs()))?;

        for message in messages {
            self.controller.publish(message);
        }

        Ok(())
    }

    fn take_wrapper(&self) -> HydrusResult<DBCursorTransactionWrapper> {
        lock(&self.state)
            .wrapper
            .take()
            .ok_or_else(|| access_error("The database is not connected!"))
    }

    fn with_taken_wrapper<T>(
        &self,
        func: impl FnOnce(&mut DBCursorTransactionWrapper) -> HydrusResult<T>,
    ) -> HydrusResult<T> {
        let mut wrapper = self.take_wrapper()?;

        let result = func(&mut wrapper);

        lock(&self.state).wrapper = Some(wrapper);

        result
    }

    /// `_set_current_status` + `publish_status_update`
    fn set_current_status(&self, status: &str) {
        {
            let mut state = lock(&self.state);

            state.current_status = status.to_string();
        }

        self.publish_status_update();
    }

    fn publish_status_update(&self) {
        let (status, job_name) = {
            let state = lock(&self.state);

            (state.current_status.clone(), state.current_job_name.clone())
        };

        self.hooks.publish_status_update(&status, &job_name);
    }

    fn set_idle(&self) {
        *lock(&self.idle) = true;

        self.idle_changed.notify_all();
    }

    fn clear_idle(&self) {
        *lock(&self.idle) = false;
    }

    fn queue_empty(&self) -> bool {
        lock(&self.queue).is_empty()
    }

    fn put_job(&self, job: Arc<DBJob>) -> HydrusResult<()> {
        {
            let mut queue = lock(&self.queue);

            // Check and push under the same lock as the shutdown drain, so a
            // job can never slip in after the queue has been drained.
            if self.loop_finished.load(Ordering::Relaxed) {
                return Err(HydrusException::new(
                    ErrorKind::Shutdown,
                    "The database is shutting down!",
                ));
            }

            queue.push_back(QueueItem::Job(job));
        }

        self.clear_idle();

        self.queue_changed.notify_all();

        Ok(())
    }

    // Public database helpers, matching `DBBase`'s methods through `self._c`.

    pub fn connection(&self) -> Option<SharedConnection> {
        lock(&self.state).connection.clone()
    }

    pub fn db_base(&self) -> HydrusResult<DBBase> {
        self.connection()
            .map(DBBase::new)
            .ok_or_else(|| access_error("The database is not connected!"))
    }

    /// `_execute`
    pub fn execute(&self, query: &str, query_args: &[&dyn rusqlite::ToSql]) -> HydrusResult<usize> {
        self.db_base()?.execute(query, query_args)
    }

    /// `_execute_many`
    pub fn execute_many(
        &self,
        query: &str,
        rows: impl IntoIterator<Item = Vec<rusqlite::types::Value>>,
    ) -> HydrusResult<usize> {
        self.db_base()?.execute_many(query, rows)
    }

    /// `_execute_cancellable`
    pub fn execute_cancellable<T>(
        &self,
        query: &str,
        query_args: &[&dyn rusqlite::ToSql],
        cancelled_hook: Option<&dyn Fn() -> bool>,
        mapper: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    ) -> HydrusResult<Vec<T>> {
        self.db_base()?
            .execute_cancellable(query, query_args, cancelled_hook, mapper)
    }

    /// `_get_last_row_id`
    pub fn get_last_row_id(&self) -> i64 {
        self.db_base().map(|db| db.get_last_row_id()).unwrap_or(0)
    }

    /// `_get_row_count`
    pub fn get_row_count(&self) -> usize {
        self.db_base().map(|db| db.get_row_count()).unwrap_or(0)
    }

    /// `_get_sum_result`
    pub fn get_sum_result(&self, result: Option<Option<i64>>) -> i64 {
        result.flatten().unwrap_or(0)
    }

    // Public API that mirrors the rest of HydrusDB.py.

    pub fn get_approx_total_file_size(&self) -> u64 {
        let mut total = 0;

        for filename in self.db_filenames.values() {
            let db_path = self.config.db_dir.join(filename);

            if let Ok(metadata) = std::fs::metadata(db_path) {
                total += metadata.len();
            }
        }

        total
    }

    pub fn get_safe_transaction_disk_space_and_current_free_space(&self) -> (u64, Option<u64>) {
        let total_db_size = self.get_approx_total_file_size();

        let space_wanted = std::cmp::min(total_db_size / 2, 5 * 1024 * 1_048_576);

        let space_wanted = std::cmp::max(space_wanted, 64 * 1_048_576);

        let free_space = hydrus_paths::get_free_space(&self.config.db_dir);

        (space_wanted, free_space)
    }

    pub fn get_ssl_paths(&self) -> HydrusResult<(PathBuf, PathBuf)> {
        let cert_here = self.ssl_cert_path.exists();
        let key_here = self.ssl_key_path.exists();

        if cert_here != key_here {
            return Err(HydrusException::new(
                ErrorKind::Unknown,
                format!(
                    "While creating the server database, only one of the paths \"{}\" and \"{}\" existed. You can create a db with these files already in place, but please either delete the existing file (to have hydrus generate its own pair) or find the other in the pair (to use your own).",
                    self.ssl_cert_path.display(),
                    self.ssl_key_path.display()
                ),
            ));
        }

        if !cert_here && !key_here {
            hydrus_data::safe_print("Generating new cert/key files.");

            if !hydrus_encryption::CRYPTO_OK {
                return Err(HydrusException::new(
                    ErrorKind::Unknown,
                    "The database was asked for ssl cert and keys to start either the server or the client api in https. The files do not exist yet, so the database wanted to create new ones, but unfortunately \"cryptography\" library is not available, so this cannot be done. If you are running from source, please install this module using pip. Or drop in your own client.crt/client.key or server.crt/server.key files in the db directory.",
                ));
            }

            hydrus_encryption::generate_open_ssl_cert_and_key_file(
                &self.ssl_cert_path,
                &self.ssl_key_path,
            )?;
        }

        Ok((self.ssl_cert_path.clone(), self.ssl_key_path.clone()))
    }

    /// `_get_possible_additional_db_filenames`
    pub fn get_possible_additional_db_filenames(&self) -> Vec<String> {
        self.config.additional_db_filenames.clone()
    }

    /// `_shrink_memory`
    pub fn shrink_memory(&self) -> HydrusResult<()> {
        self.db_base()?.execute_batch("PRAGMA shrink_memory;")
    }

    pub fn get_status(&self) -> (String, String) {
        let state = lock(&self.state);

        (state.current_status.clone(), state.current_job_name.clone())
    }

    pub fn is_connected(&self) -> bool {
        lock(&self.state).is_connected
    }

    pub fn is_db_updated(&self) -> bool {
        self.is_db_updated.load(Ordering::Relaxed)
    }

    pub fn is_first_start(&self) -> bool {
        self.is_first_start.load(Ordering::Relaxed)
    }

    pub fn loop_is_finished(&self) -> bool {
        self.loop_finished.load(Ordering::Relaxed)
    }

    pub fn jobs_queue_empty(&self) -> bool {
        self.queue_empty()
    }

    pub fn currently_doing_job(&self) -> bool {
        lock(&self.state).currently_doing_job
    }

    pub fn ready_to_serve_requests(&self) -> bool {
        lock(&self.state).ready_to_serve_requests
    }

    /// `read( action, *args, **kwargs )`
    pub fn read(&self, action: &str) -> HydrusResult<Box<dyn Any + Send>> {
        let job_type = if self
            .config
            .read_write_actions
            .iter()
            .any(|read_write_action| read_write_action == action)
        {
            "read_write"
        } else {
            "read"
        };

        let job = Arc::new(DBJob::new(job_type, true, action));

        self.put_job(job.clone())?;

        job.get_result()
    }

    /// `write( action, synchronous, *args, **kwargs )`
    ///
    /// A synchronous write returns its value; an asynchronous one returns
    /// `None`, matching Python.
    pub fn write(
        &self,
        action: &str,
        synchronous: bool,
    ) -> HydrusResult<Option<Box<dyn Any + Send>>> {
        let job = Arc::new(DBJob::new("write", synchronous, action));

        self.put_job(job.clone())?;

        if synchronous {
            job.get_result().map(Some)
        } else {
            Ok(None)
        }
    }

    /// `force_a_commit`
    pub fn force_a_commit(&self) -> HydrusResult<()> {
        let in_transaction = {
            let state = lock(&self.state);

            match state.wrapper.as_ref() {
                None => return Ok(()),
                Some(wrapper) => wrapper.in_transaction(),
            }
        };

        if in_transaction {
            self.with_taken_wrapper(|wrapper| {
                wrapper.do_a_commit_as_soon_as_possible();

                Ok(())
            })?;

            self.write("null", true)?;
        }

        Ok(())
    }

    pub fn schedule_commit_check(&self) {
        {
            lock(&self.queue).push_back(QueueItem::CommitCheck);
        }

        self.queue_changed.notify_all();
    }

    pub fn shutdown(&self) {
        self.local_shutdown.store(true, Ordering::Relaxed);

        {
            lock(&self.queue).push_back(QueueItem::Shutdown);
        }

        self.queue_changed.notify_all();
    }

    pub fn wait_until_free(&self) {
        let mut idle = lock(&self.idle);

        while !*idle {
            idle = self
                .idle_changed
                .wait(idle)
                .unwrap_or_else(|error| error.into_inner());
        }
    }

    pub fn pause_and_disconnect(&self, pause_and_disconnect: bool) {
        self.pause_and_disconnect
            .store(pause_and_disconnect, Ordering::Relaxed);
    }
}

impl Drop for HydrusDB {
    fn drop(&mut self) {
        let on_database_thread =
            lock(&self.thread_id).as_ref() == Some(&std::thread::current().id());

        self.local_shutdown.store(true, Ordering::Relaxed);

        self.queue_changed.notify_all();

        if on_database_thread {
            // The last owner went away while the database thread was between
            // jobs. Clean up here, since joining our own thread is impossible.
            if !self.loop_finished.load(Ordering::Relaxed) {
                self.finish_loop();
            }

            return;
        }

        let handle = lock(&self.thread).take();

        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}

fn is_safe_identifier(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(|character: char| character.is_ascii_digit())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    use crate::hydrus_db_base::JobDatabase;
    use crate::hydrus_temp;

    /// Keep DB tests from clearing the process-global temporary table name
    /// cache while another test has a name checked out.
    fn lock_temp_tables() -> std::sync::MutexGuard<'static, ()> {
        crate::hydrus_db_base::TEMP_TABLE_TEST_LOCK
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn temp_dir(name: &str) -> PathBuf {
        hydrus_temp::get_sub_temp_dir(&format!("hydrus_db_{name}")).expect("temp dir")
    }

    fn remove_db_dir(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Records status updates so tests can prove the hook is called.
    struct TestHooks {
        status_updates: Arc<Mutex<Vec<(String, String)>>>,
        write_command_calls: Arc<AtomicUsize>,
    }

    impl TestHooks {
        fn new() -> Self {
            Self {
                status_updates: Arc::new(Mutex::new(Vec::new())),
                write_command_calls: Arc::new(AtomicUsize::new(0)),
            }
        }
    }

    fn count_command(db: &DBBase) -> DbCommand {
        let db = db.clone();

        Arc::new(move || {
            let count = db
                .query_one("SELECT COUNT( * ) FROM things;", &[], |row| {
                    row.get::<_, i64>(0)
                })?
                .unwrap_or(0);

            Ok(Box::new(count) as Box<dyn Any + Send>)
        })
    }

    fn add_command(db: &DBBase, calls: Arc<AtomicUsize>) -> DbCommand {
        let db = db.clone();

        Arc::new(move || {
            db.execute("INSERT INTO things ( value ) VALUES ( 1 );", &[])?;

            calls.fetch_add(1, Ordering::Relaxed);

            Ok(Box::new(()) as Box<dyn Any + Send>)
        })
    }

    fn fail_command() -> DbCommand {
        Arc::new(|| {
            Err(HydrusException::new(
                ErrorKind::Unknown,
                "deliberate failure",
            ))
        })
    }

    impl DBHooks for TestHooks {
        fn create_db(
            &self,
            db: &DBBase,
            _cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
        ) -> HydrusResult<()> {
            db.execute("CREATE TABLE version ( version INTEGER );", &[])?;

            db.execute(
                "INSERT INTO version ( version ) VALUES ( ? );",
                &[&(SOFTWARE_VERSION as i64)],
            )?;

            db.execute("CREATE TABLE things ( value INTEGER );", &[])?;

            Ok(())
        }

        fn init_commands(&self, db: &DBBase) -> HydrusResult<(DbCommands, DbCommands)> {
            let read_commands = DbCommands::from([("count".to_string(), count_command(db))]);

            let write_commands = DbCommands::from([
                (
                    "add".to_string(),
                    add_command(db, self.write_command_calls.clone()),
                ),
                ("fail".to_string(), fail_command()),
                ("null".to_string(), null_db_command()),
            ]);

            Ok((read_commands, write_commands))
        }

        fn publish_status_update(&self, status: &str, job_name: &str) {
            self.status_updates
                .lock()
                .unwrap()
                .push((status.to_string(), job_name.to_string()));
        }
    }

    fn wait_for_loop_finished(db: &HydrusDB) {
        let started = Instant::now();

        while !db.loop_is_finished() && started.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(10));
        }

        assert!(db.loop_is_finished(), "the db loop did not finish");
    }

    #[test]
    fn approx_durations_and_vacuum_paths() {
        let _guard = lock_temp_tables();

        assert_eq!(get_approx_vacuum_duration(1_048_576), 1);
        assert_eq!(get_approx_vacuum_duration(0), 0);
        assert_eq!(get_approx_vacuum_into_duration(10 * 1_048_576), 1);
        assert_eq!(get_approx_vacuum_into_duration(0), 0);

        let path = Path::new("example.db");

        assert_eq!(
            get_vacuum_paths(path),
            (
                PathBuf::from("example.db.prevacuum"),
                PathBuf::from("example.db.vacuum")
            )
        );
    }

    #[test]
    fn check_can_vacuum_reports_the_important_numbers() {
        let _guard = lock_temp_tables();

        let dir = temp_dir("vacuum_check");
        let path = dir.join("test.db");

        let connection = Connection::open(&path).expect("db");

        connection
            .execute_batch(
                "CREATE TABLE data ( value BLOB ); \
                 INSERT INTO data VALUES ( randomblob( 10000 ) ); \
                 DELETE FROM data;",
            )
            .expect("data");

        check_can_vacuum(&path, None).expect("can vacuum");
        check_can_vacuum_into(&path, None).expect("can vacuum into");

        let db = DBBase::open(&path).expect("db base");

        check_can_vacuum_cursor(&path, &db, None).expect("can vacuum cursor");
        check_can_vacuum_into_cursor(&path, &db, None).expect("can vacuum into cursor");

        drop(db);
        drop(connection);
        remove_db_dir(&dir);
    }

    #[test]
    fn vacuum_db_keeps_the_data() {
        let _guard = lock_temp_tables();

        let dir = temp_dir("vacuum");
        let path = dir.join("test.db");

        let connection = Connection::open(&path).expect("db");

        connection
            .execute_batch(
                "CREATE TABLE data ( value INTEGER ); \
                 WITH RECURSIVE counter ( n ) AS ( SELECT 1 UNION ALL SELECT n + 1 FROM counter WHERE n < 1000 ) \
                 INSERT INTO data SELECT n FROM counter; \
                 DELETE FROM data WHERE value > 10;",
            )
            .expect("data");

        drop(connection);

        vacuum_db(&path).expect("vacuum");

        let connection = Connection::open(&path).expect("reopen");

        let count = connection
            .query_row("SELECT COUNT( * ) FROM data;", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count");

        assert_eq!(count, 10);

        drop(connection);
        remove_db_dir(&dir);
    }

    #[test]
    fn vacuum_db_into_replaces_the_original() {
        let _guard = lock_temp_tables();

        let dir = temp_dir("vacuum_into");
        let path = dir.join("test.db");

        let connection = Connection::open(&path).expect("db");

        connection
            .execute_batch(
                "CREATE TABLE data ( value INTEGER ); \
                 INSERT INTO data VALUES ( 1 ), ( 2 ), ( 3 ), ( 4 ), ( 5 );",
            )
            .expect("data");

        drop(connection);

        vacuum_db_into(&path).expect("vacuum into");

        assert!(path.exists());
        assert!(!path_with_suffix(&path, ".prevacuum").exists());
        assert!(!path_with_suffix(&path, ".vacuum").exists());

        let connection = Connection::open(&path).expect("reopen");

        let count = connection
            .query_row("SELECT COUNT( * ) FROM data;", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count");

        assert_eq!(count, 5);

        drop(connection);
        remove_db_dir(&dir);
    }

    #[test]
    fn vacuum_db_into_refuses_stale_sidecars() {
        let _guard = lock_temp_tables();

        let dir = temp_dir("vacuum_sidecar");
        let path = dir.join("test.db");

        Connection::open(&path).expect("db");

        std::fs::write(path_with_suffix(&path, "-wal"), b"stale").expect("sidecar");

        let error = vacuum_db_into(&path).expect_err("sidecar should block");

        assert!(error.to_string().contains("WAL"));

        let _ = std::fs::remove_file(path_with_suffix(&path, "-wal"));

        remove_db_dir(&dir);
    }

    #[test]
    fn large_id_queries_are_chunked() {
        let _guard = lock_temp_tables();

        let connection = Connection::open_in_memory().expect("db");

        connection
            .execute_batch(
                "CREATE TABLE numbers ( id INTEGER ); \
                 WITH RECURSIVE counter ( n ) AS ( SELECT 0 UNION ALL SELECT n + 1 FROM counter WHERE n < 9 ) \
                 INSERT INTO numbers SELECT n FROM counter;",
            )
            .expect("data");

        let chunks =
            read_large_id_query_in_separate_chunks(&connection, "SELECT id FROM numbers;", 4)
                .expect("chunks");

        // Python's `job_id` starts at 1 while the loop starts at 0, so the first
        // chunk is one row short.
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], (vec![0, 1, 2], 3, 10));
        assert_eq!(chunks[1], (vec![3, 4, 5, 6], 7, 10));
        assert_eq!(chunks[2], (vec![7, 8, 9], 10, 10));

        assert!(
            read_large_id_query_in_separate_chunks(&connection, "SELECT id FROM numbers;", 0)
                .is_err()
        );
    }

    #[test]
    fn db_lifecycle_runs_jobs_and_shuts_down() {
        let _guard = lock_temp_tables();

        let dir = temp_dir("lifecycle");
        let hooks = TestHooks::new();
        let status_updates = hooks.status_updates.clone();

        let mut config = HydrusDBConfig::new(&dir, "test");
        config.update_wait_seconds = 0;

        let db = HydrusDB::from_parts(config, hooks, NullDBController).expect("db");

        assert!(db.is_first_start());
        assert!(db.is_connected());
        assert!(db.ready_to_serve_requests());
        assert!(!db.is_db_updated());

        let count = db.read("count").expect("count");

        assert_eq!(count.downcast_ref::<i64>(), Some(&0));

        db.write("add", true).expect("sync write").expect("value");
        db.write("add", false).expect("async write");

        db.wait_until_free();

        let count = db.read("count").expect("count");

        assert_eq!(count.downcast_ref::<i64>(), Some(&2));

        let error = db.read("missing").expect_err("unknown read");

        assert!(error.to_string().contains("unknown read command"));

        let error = db.write("missing", true).expect_err("unknown write");

        assert!(error.to_string().contains("unknown write command"));

        let error = db.write("fail", true).expect_err("failing command");

        assert!(error.to_string().contains("deliberate failure"));

        db.force_a_commit().expect("force commit");

        assert!(!status_updates.lock().unwrap().is_empty());

        db.shutdown();

        wait_for_loop_finished(&db);

        assert!(!db.is_connected());
        assert!(db.jobs_queue_empty());

        let error = db.read("count").expect_err("shut down");

        assert!(error.is_a(ErrorKind::Shutdown));

        drop(db);
        remove_db_dir(&dir);
    }

    #[test]
    fn dropping_the_last_reference_cleans_up_the_thread() {
        let _guard = lock_temp_tables();

        let dir = temp_dir("drop");

        {
            let mut config = HydrusDBConfig::new(&dir, "test");
            config.update_wait_seconds = 0;

            let db = HydrusDB::from_parts(config, TestHooks::new(), NullDBController).expect("db");

            let count = db.read("count").expect("count");

            assert_eq!(count.downcast_ref::<i64>(), Some(&0));

            // Deliberately no shutdown: dropping the last reference must stop
            // the database thread and clean up the durable temporary db.
        }

        let durable_temp_path = dir.join("test.temp.db");
        let started = Instant::now();

        while durable_temp_path.exists() && started.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(10));
        }

        assert!(!durable_temp_path.exists());

        remove_db_dir(&dir);
    }

    #[test]
    fn external_databases_are_attached_and_sized() {
        let _guard = lock_temp_tables();

        let dir = temp_dir("external");

        struct ExternalHooks;

        impl DBHooks for ExternalHooks {
            fn create_db(
                &self,
                db: &DBBase,
                _cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
            ) -> HydrusResult<()> {
                db.execute("CREATE TABLE version ( version INTEGER );", &[])?;
                db.execute(
                    "INSERT INTO version ( version ) VALUES ( ? );",
                    &[&(SOFTWARE_VERSION as i64)],
                )?;

                db.execute("CREATE TABLE ext.things ( value INTEGER );", &[])?;
                db.execute("INSERT INTO ext.things ( value ) VALUES ( 7 );", &[])?;

                Ok(())
            }
        }

        let mut config = HydrusDBConfig::new(&dir, "test");
        config.update_wait_seconds = 0;
        config.external_databases = vec![("ext".to_string(), "test_ext.db".to_string())];

        let db = HydrusDB::from_parts(config, ExternalHooks, NullDBController).expect("db");

        assert!(dir.join("test_ext.db").exists());
        assert!(dir.join("test.temp.db").exists());
        assert!(db.get_approx_total_file_size() > 0);

        db.shutdown();
        wait_for_loop_finished(&db);

        drop(db);

        let connection = Connection::open(dir.join("test_ext.db")).expect("external db");

        let value = connection
            .query_row("SELECT value FROM things;", [], |row| row.get::<_, i64>(0))
            .expect("value");

        assert_eq!(value, 7);

        drop(connection);
        remove_db_dir(&dir);
    }

    struct UpdateHooks {
        start_version: i64,
        updates: Arc<AtomicUsize>,
    }

    impl DBHooks for UpdateHooks {
        fn create_db(
            &self,
            db: &DBBase,
            _cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
        ) -> HydrusResult<()> {
            db.execute("CREATE TABLE version ( version INTEGER );", &[])?;
            db.execute(
                "INSERT INTO version ( version ) VALUES ( ? );",
                &[&self.start_version],
            )?;

            Ok(())
        }

        fn update_db(
            &self,
            db: &DBBase,
            _cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
            version: i64,
        ) -> HydrusResult<()> {
            assert_eq!(
                version,
                self.start_version + self.updates.load(Ordering::Relaxed) as i64
            );

            db.execute("UPDATE version SET version = version + 1;", &[])?;

            self.updates.fetch_add(1, Ordering::Relaxed);

            Ok(())
        }
    }

    #[test]
    fn db_updates_to_the_software_version() {
        let _guard = lock_temp_tables();

        let dir = temp_dir("update");
        let updates = Arc::new(AtomicUsize::new(0));

        let mut config = HydrusDBConfig::new(&dir, "test");
        config.update_wait_seconds = 0;

        let db = HydrusDB::from_parts(
            config,
            UpdateHooks {
                start_version: SOFTWARE_VERSION as i64 - 1,
                updates: updates.clone(),
            },
            NullDBController,
        )
        .expect("db");

        assert!(db.is_db_updated());
        assert_eq!(updates.load(Ordering::Relaxed), 1);

        let version = db
            .db_base()
            .unwrap()
            .query_one("SELECT version FROM version;", &[], |row| {
                row.get::<_, i64>(0)
            })
            .expect("version")
            .expect("row");

        assert_eq!(version, SOFTWARE_VERSION as i64);

        db.shutdown();
        wait_for_loop_finished(&db);

        drop(db);
        remove_db_dir(&dir);
    }

    #[test]
    fn missing_version_table_is_fatal() {
        let _guard = lock_temp_tables();

        let dir = temp_dir("missing_version");
        let path = dir.join("test.db");

        {
            let connection = Connection::open(&path).expect("db");

            connection
                .execute_batch("CREATE TABLE unrelated ( id INTEGER );")
                .expect("table");
        }

        let mut config = HydrusDBConfig::new(&dir, "test");
        config.update_wait_seconds = 0;

        let error = HydrusDB::from_parts(config, BaseDBHooks, NullDBController)
            .err()
            .expect("missing version should fail");

        assert!(error.is_a(ErrorKind::DbAccess));
        assert!(error.to_string().contains("version"));

        remove_db_dir(&dir);
    }

    #[test]
    fn read_write_actions_use_the_read_map() {
        let _guard = lock_temp_tables();

        let dir = temp_dir("read_write");

        struct ReadWriteHooks;

        impl DBHooks for ReadWriteHooks {
            fn create_db(
                &self,
                db: &DBBase,
                _cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
            ) -> HydrusResult<()> {
                db.execute("CREATE TABLE version ( version INTEGER );", &[])?;
                db.execute(
                    "INSERT INTO version ( version ) VALUES ( ? );",
                    &[&(SOFTWARE_VERSION as i64)],
                )?;

                Ok(())
            }

            fn init_commands(&self, db: &DBBase) -> HydrusResult<(DbCommands, DbCommands)> {
                let read_db = db.clone();

                let read_commands = DbCommands::from([(
                    "both".to_string(),
                    Arc::new(move || {
                        read_db.execute("CREATE TABLE IF NOT EXISTS extra ( id INTEGER );", &[])?;

                        Ok(Box::new("both") as Box<dyn Any + Send>)
                    }) as DbCommand,
                )]);

                Ok((read_commands, DbCommands::new()))
            }
        }

        let mut config = HydrusDBConfig::new(&dir, "test");
        config.update_wait_seconds = 0;
        config.read_write_actions = vec!["both".to_string()];

        let db = HydrusDB::from_parts(config, ReadWriteHooks, NullDBController).expect("db");

        let value = db.read("both").expect("read_write command");

        assert_eq!(value.downcast_ref::<&str>(), Some(&"both"));

        db.wait_until_free();

        db.shutdown();
        wait_for_loop_finished(&db);

        drop(db);
        remove_db_dir(&dir);
    }

    #[test]
    fn job_database_still_hands_results_across() {
        let _guard = lock_temp_tables();

        // The older `JobDatabase` remains the primitive for callers that want
        // to run a closure directly; make sure the two job types agree on the
        // synchronous result protocol.
        let job = JobDatabase::new("read", true, "thing", || {
            Ok(Box::new(7i64) as Box<dyn Any + Send>)
        });

        let action = job.take_action().expect("action");

        job.put_result(action());

        assert_eq!(job.get_result().unwrap().downcast_ref::<i64>(), Some(&7));
    }
}
