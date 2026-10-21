//! Rust port of `hydrus/core/HydrusDBBase.py`.
//!
//! Python gives every db module a `sqlite3.Cursor` from one shared connection.
//! Rust cannot share a `rusqlite::Connection` by reference across owners, so the
//! port shares `Arc<Mutex<Connection>>`. Every helper locks the connection for
//! the duration of one statement; SQLite's transaction state lives on the
//! connection, so transactions still span many calls exactly as in Python.
//!
//! `TemporaryIntegerTable` performs its setup on construction and cleans up on
//! `Drop`, which is the RAII spelling of Python's context manager.

use std::collections::HashMap;
use std::fmt::Display;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};

use rusqlite::{Connection, OptionalExtension, Row, ToSql};
use serde_json::{Map, Value};

use crate::hydrus_data;
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};
use crate::hydrus_globals;
use crate::hydrus_paths;
use crate::hydrus_profiling;
use crate::hydrus_pubsub::PubSubMessage;
use crate::hydrus_time;
use crate::hydrus_temp;

/// The connection every db object in a process shares.
pub type SharedConnection = Arc<Mutex<Connection>>;

pub const JOURNAL_SIZE_LIMIT: u64 = 128 * 1024 * 1024;
pub const JOURNAL_ZERO_PERIOD: i64 = 900;
pub const MEM_REFRESH_PERIOD: i64 = 600;
pub const WAL_PASSIVE_CHECKPOINT_PERIOD: i64 = 300;
pub const WAL_TRUNCATE_CHECKPOINT_PERIOD: i64 = 900;

fn db_error(context: impl Display, error: rusqlite::Error) -> HydrusException {
    HydrusException::new(ErrorKind::Db, format!("{context}: {error}"))
}

fn access_error(message: impl Into<String>) -> HydrusException {
    HydrusException::new(ErrorKind::DbAccess, message)
}

fn lock_connection(connection: &SharedConnection) -> MutexGuard<'_, Connection> {
    connection.lock().unwrap_or_else(|error| error.into_inner())
}

/// `check_has_space_for_db_transaction( db_dir, num_bytes, no_temp_needed = False )`
pub fn check_has_space_for_db_transaction(
    db_dir: &Path,
    num_bytes: u64,
    no_temp_needed: bool,
) -> HydrusResult<()> {
    let (temp_space_needed, destination_space_needed) = if no_temp_needed {
        (0, num_bytes + num_bytes / 10)
    } else {
        let temp_space_needed = num_bytes + num_bytes / 10;

        (temp_space_needed, temp_space_needed * 2)
    };

    if hydrus_globals::no_db_temp_files() {
        // PSUTIL_OK is false in this port, so the Python memory check is a
        // no-op here too.

        let db_disk_free_space = hydrus_paths::get_free_space(db_dir);

        if let Some(free_space) = db_disk_free_space
            && free_space < destination_space_needed
        {
            return Err(HydrusException::new(
                ErrorKind::Unknown,
                format!(
                    "I believe you need about {} on your db's disk partition (perhaps only temporarily), but you only seem to have {}.",
                    hydrus_data::to_human_bytes(destination_space_needed),
                    hydrus_data::to_human_bytes(free_space)
                ),
            ));
        }
    } else {
        let temp_dir = hydrus_temp::get_current_sqlite_temp_dir();

        let temp_disk_free_space = hydrus_paths::get_free_space(&temp_dir);

        let temp_and_db_on_same_device = hydrus_paths::get_device_id(&temp_dir).ok()
            == hydrus_paths::get_device_id(db_dir).ok();

        if temp_and_db_on_same_device && temp_space_needed > 0 {
            let space_needed = temp_space_needed + destination_space_needed;

            if let Some(free_space) = temp_disk_free_space
                && free_space < space_needed
            {
                return Err(HydrusException::new(
                    ErrorKind::Unknown,
                    format!(
                        "I believe you need about {} on your db's disk partition (perhaps only temporarily), which I think also holds your SQLite temporary path ({}), but you only seem to have {}.",
                        hydrus_data::to_human_bytes(space_needed),
                        temp_dir.display(),
                        hydrus_data::to_human_bytes(free_space)
                    ),
                ));
            }
        } else {
            if let Some(free_space) = temp_disk_free_space
                && free_space < temp_space_needed
            {
                let mut message = format!(
                    "I believe you need about {} free on the disk partition holding your SQLite temporary path, which I think is \"{}\", but you only seem to have {}.",
                    hydrus_data::to_human_bytes(temp_space_needed),
                    temp_dir.display(),
                    hydrus_data::to_human_bytes(free_space)
                );

                let temp_total_space = hydrus_paths::get_total_space(&temp_dir);

                if let Some(total_space) = temp_total_space
                    && total_space <= 4 * 1024 * 1024 * 1024
                {
                    message += " I think you might be using a ramdisk! You may want to instead launch hydrus with a different temp dir. Please check the \"launch arguments\" section of the help.";
                } else {
                    message += " Please note that temporary paths can be complicated, and if you have a ramdisk or OS settings limiting how large it can get, or you simply cannot free space on your system drive, you may want to instead launch hydrus with a different temp directory. Please check the \"launch arguments\" section of the help.";
                }

                return Err(HydrusException::new(ErrorKind::Unknown, message));
            }

            let db_disk_free_space = hydrus_paths::get_free_space(db_dir);

            if let Some(free_space) = db_disk_free_space
                && free_space < destination_space_needed
            {
                return Err(HydrusException::new(
                    ErrorKind::Unknown,
                    format!(
                        "I believe you need about {} on your db's disk partition (perhaps only temporarily), but you only seem to have {}.",
                        hydrus_data::to_human_bytes(destination_space_needed),
                        hydrus_data::to_human_bytes(free_space)
                    ),
                ));
            }
        }
    }

    Ok(())
}

/// `read_from_cancellable_cursor( cursor, largest_group_size, cancelled_hook = None )`
///
/// Rust iterators are already pull-based, so this simply collects in growing
/// batches and stops as soon as the hook fires.
pub fn read_from_cancellable_cursor<T, I>(
    cursor: I,
    largest_group_size: usize,
    cancelled_hook: Option<&dyn Fn() -> bool>,
) -> Vec<T>
where
    I: IntoIterator<Item = T>,
{
    let mut results = Vec::new();

    if let Some(hook) = cancelled_hook
        && hook()
    {
        return results;
    }

    let _ = largest_group_size;

    for item in cursor {
        results.push(item);

        if let Some(hook) = cancelled_hook
            && hook()
        {
            break;
        }
    }

    results
}

#[derive(Default)]
struct TemporaryNameState {
    names: HashMap<Vec<String>, Vec<String>>,
    counters: HashMap<Vec<String>, u64>,
}

/// Python's `TemporaryIntegerTableNameCache`, a process-global singleton.
pub struct TemporaryIntegerTableNameCache {
    state: Mutex<TemporaryNameState>,
}

impl TemporaryIntegerTableNameCache {
    /// `TemporaryIntegerTableNameCache.instance()`
    pub fn instance() -> &'static TemporaryIntegerTableNameCache {
        static INSTANCE: OnceLock<TemporaryIntegerTableNameCache> = OnceLock::new();

        INSTANCE.get_or_init(|| TemporaryIntegerTableNameCache {
            state: Mutex::new(TemporaryNameState::default()),
        })
    }

    fn lock(&self) -> MutexGuard<'_, TemporaryNameState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    /// `clear`
    pub fn clear(&self) {
        let mut state = self.lock();

        state.names.clear();
        state.counters.clear();
    }

    /// `get_name( column_names )`. Returns `(initialised, table_name)`.
    pub fn get_name<S: AsRef<str>>(&self, column_names: &[S]) -> (bool, String) {
        let key = column_names
            .iter()
            .map(|name| name.as_ref().to_string())
            .collect::<Vec<_>>();

        let mut state = self.lock();

        if let Some(table_name) = state.names.get_mut(&key).and_then(Vec::pop) {
            return (true, table_name);
        }

        let counter = *state.counters.get(&key).unwrap_or(&0);

        let table_name = format!(
            "mem.temp_{}_int_{}_{}",
            key.len(),
            key.join("_"),
            counter
        );

        // Python appends then immediately pops the freshly made name, so it is
        // handed out without entering the free list.
        state.counters.insert(key, counter + 1);

        (false, table_name)
    }

    /// `release_name( column_names, table_name )`
    pub fn release_name<S: AsRef<str>>(&self, column_names: &[S], table_name: &str) {
        let key = column_names
            .iter()
            .map(|name| name.as_ref().to_string())
            .collect::<Vec<_>>();

        let mut state = self.lock();

        state
            .names
            .entry(key)
            .or_default()
            .push(table_name.to_string());
    }
}

/// Python's `TemporaryIntegerTable`. Construction performs `__enter__`; `Drop`
/// performs `__exit__`.
pub struct TemporaryIntegerTable {
    connection: SharedConnection,
    table_name: String,
    column_names: Vec<String>,
}

impl TemporaryIntegerTable {
    /// Single-column table.
    pub fn new(
        connection: SharedConnection,
        integers: &[i64],
        column_name: &str,
    ) -> HydrusResult<Self> {
        // Python converts the iterable to a set, so duplicate ids collapse.
        let unique_integers = integers.iter().copied().collect::<std::collections::BTreeSet<_>>();

        let rows = unique_integers
            .into_iter()
            .map(|integer| vec![integer])
            .collect::<Vec<_>>();

        Self::with_rows(connection, &rows, &[column_name])
    }

    /// Multi-column table, matching the tuple form of Python's context manager.
    pub fn with_rows(
        connection: SharedConnection,
        rows: &[Vec<i64>],
        column_names: &[&str],
    ) -> HydrusResult<Self> {
        if column_names.is_empty() {
            return Err(db_error(
                "A temporary integer table needs at least one column",
                rusqlite::Error::InvalidQuery,
            ));
        }

        let (initialised, table_name) =
            TemporaryIntegerTableNameCache::instance().get_name(column_names);

        let table = Self {
            connection,
            table_name,
            column_names: column_names.iter().map(|name| name.to_string()).collect(),
        };

        // Python also folds the iterable of tuples into a set.
        let unique_rows = rows.iter().cloned().collect::<std::collections::BTreeSet<_>>();

        {
            let connection = lock_connection(&table.connection);

            if !initialised {
            if table.column_names.len() == 1 {
                connection
                    .execute(
                        &format!(
                            "CREATE TABLE IF NOT EXISTS {} ( {} INTEGER PRIMARY KEY );",
                            table.table_name, table.column_names[0]
                        ),
                        [],
                    )
                    .map_err(|error| db_error("Could not create a temporary integer table", error))?;
            } else {
                let column_defs = table
                    .column_names
                    .iter()
                    .map(|name| format!("{name} INTEGER"))
                    .collect::<Vec<_>>()
                    .join(", ");

                let columns_listed = table.column_names.join(", ");

                connection
                    .execute(
                        &format!(
                            "CREATE TABLE IF NOT EXISTS {} ( {}, PRIMARY KEY ( {} ) );",
                            table.table_name, column_defs, columns_listed
                        ),
                        [],
                    )
                    .map_err(|error| db_error("Could not create a temporary integer table", error))?;

                let table_name_simple = table
                    .table_name
                    .split_once('.')
                    .map(|(_, simple)| simple)
                    .unwrap_or(&table.table_name);

                for column_name in &table.column_names {
                    let index_name = format!("{}_{}_index", table.table_name, column_name);

                    connection
                        .execute(
                            &format!(
                                "CREATE INDEX IF NOT EXISTS {index_name} ON {table_name_simple} ({column_name});"
                            ),
                            [],
                        )
                        .map_err(|error| {
                            db_error("Could not create a temporary integer table index", error)
                        })?;
                }
            }
        }

        let column_tup = table.column_names.join(", ");
        let qmark_tup = (0..table.column_names.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(", ");

        let mut statement = connection
            .prepare(&format!(
                "INSERT INTO {} ( {column_tup} ) VALUES ( {qmark_tup} );",
                table.table_name
            ))
            .map_err(|error| db_error("Could not prepare the temporary table insert", error))?;

        for row in &unique_rows {
            if row.len() != table.column_names.len() {
                return Err(db_error(
                    "Temporary integer table row has the wrong number of columns",
                    rusqlite::Error::InvalidQuery,
                ));
            }

            let values = row
                .iter()
                .map(|value| value as &dyn ToSql)
                .collect::<Vec<_>>();

            statement
                .execute(values.as_slice())
                .map_err(|error| db_error("Could not populate a temporary integer table", error))?;
        }
        }

        Ok(table)
    }

    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

impl Drop for TemporaryIntegerTable {
    fn drop(&mut self) {
        {
            let connection = lock_connection(&self.connection);

            let _ = connection.execute(&format!("DELETE FROM {};", self.table_name), []);
        }

        TemporaryIntegerTableNameCache::instance()
            .release_name(&self.column_names.iter().map(String::as_str).collect::<Vec<_>>(), &self.table_name);
    }
}

/// Python's `JobDatabase`, modelled with a boxed closure and a typed erasure.
///
/// Read/write commands have different return types, so results travel as
/// `Box<dyn Any + Send>` and the caller downcasts the type it asked for.
pub struct JobDatabase {
    job_type: String,
    synchronous: bool,
    name: String,
    action: Mutex<Option<Box<dyn FnOnce() -> HydrusResult<Box<dyn std::any::Any + Send>> + Send>>>,
    result: Mutex<Option<HydrusResult<Box<dyn std::any::Any + Send>>>>,
    result_ready: Condvar,
}

impl JobDatabase {
    pub fn new(
        job_type: impl Into<String>,
        synchronous: bool,
        name: impl Into<String>,
        action: impl FnOnce() -> HydrusResult<Box<dyn std::any::Any + Send>> + Send + 'static,
    ) -> Self {
        Self {
            job_type: job_type.into(),
            synchronous,
            name: name.into(),
            action: Mutex::new(Some(Box::new(action))),
            result: Mutex::new(None),
            result_ready: Condvar::new(),
        }
    }

    /// `get_callable_tuple`, in the form of taking the queued action.
    pub fn take_action(
        &self,
    ) -> Option<Box<dyn FnOnce() -> HydrusResult<Box<dyn std::any::Any + Send>> + Send>> {
        self.action
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
    }

    /// `get_result`. Waits for the db thread, then re-raises or returns.
    pub fn get_result(&self) -> HydrusResult<Box<dyn std::any::Any + Send>> {
        let mut result = self.result.lock().unwrap_or_else(|error| error.into_inner());

        while result.is_none() {
            result = self
                .result_ready
                .wait(result)
                .unwrap_or_else(|error| error.into_inner());
        }

        result.take().unwrap()
    }

    pub fn get_type(&self) -> &str {
        &self.job_type
    }

    pub fn is_synchronous(&self) -> bool {
        self.synchronous
    }

    /// `put_result( result )`
    pub fn put_result(&self, value: HydrusResult<Box<dyn std::any::Any + Send>>) {
        {
            let mut result = self.result.lock().unwrap_or_else(|error| error.into_inner());

            *result = Some(value);
        }

        self.result_ready.notify_all();
    }

    /// `to_string`
    pub fn to_string(&self) -> String {
        format!("{} {}", self.job_type, self.name)
    }
}

/// Python's `DBBase`: shared connection plus the schema helpers every module
/// uses.
#[derive(Clone)]
pub struct DBBase {
    connection: SharedConnection,
}

impl DBBase {
    pub fn new(connection: SharedConnection) -> Self {
        Self { connection }
    }

    /// Open a fresh single-owner connection (useful for standalone scripts and
    /// tests).
    pub fn open(path: impl AsRef<Path>) -> HydrusResult<Self> {
        let path = path.as_ref();

        let connection = Connection::open(path).map_err(|error| {
            db_error(
                format!("Could not open \"{}\"", path.display()),
                error,
            )
        })?;

        Ok(Self::new(Arc::new(Mutex::new(connection))))
    }

    pub fn connection(&self) -> SharedConnection {
        self.connection.clone()
    }

    fn lock(&self) -> MutexGuard<'_, Connection> {
        lock_connection(&self.connection)
    }

    fn maybe_print_query_plan(
        connection: &Connection,
        query: &str,
        query_args: &[&dyn ToSql],
    ) {
        if !hydrus_profiling::query_planner_mode() || hydrus_profiling::query_was_planned(query) {
            return;
        }

        let Ok(mut statement) = connection.prepare(&format!("EXPLAIN QUERY PLAN {query}")) else {
            return;
        };

        let Ok(rows) = statement.query_map(query_args, |row| row.get::<_, String>(3)) else {
            return;
        };

        let plan_lines = rows.filter_map(Result::ok).collect::<Vec<_>>();

        hydrus_profiling::print_query_plan(query, &plan_lines);
    }

    /// `_execute`. Returns the number of changed rows.
    pub fn execute(&self, query: &str, query_args: &[&dyn ToSql]) -> HydrusResult<usize> {
        let connection = self.lock();

        Self::maybe_print_query_plan(&connection, query, query_args);

        connection
            .execute(query, query_args)
            .map_err(|error| db_error(format!("Could not execute \"{query}\""), error))
    }

    /// Execute a semicolon-separated script, ignoring any result rows.
    ///
    /// Python's `cursor.execute` does not mind a statement that returns rows
    /// (many `PRAGMA` assignments do), so this is the safe port for those.
    pub fn execute_batch(&self, query: &str) -> HydrusResult<()> {
        let connection = self.lock();

        connection
            .execute_batch(query)
            .map_err(|error| db_error(format!("Could not execute \"{query}\""), error))
    }

    /// `_execute_many`
    pub fn execute_many(
        &self,
        query: &str,
        rows: impl IntoIterator<Item = Vec<rusqlite::types::Value>>,
    ) -> HydrusResult<usize> {
        let connection = self.lock();

        let mut statement = connection
            .prepare(query)
            .map_err(|error| db_error(format!("Could not prepare \"{query}\""), error))?;

        let mut total = 0usize;

        for row in rows {
            total += statement
                .execute(rusqlite::params_from_iter(row))
                .map_err(|error| db_error(format!("Could not execute \"{query}\""), error))?;
        }

        Ok(total)
    }

    /// `_execute_cancellable`: runs a query and stops collecting when the hook
    /// fires.
    pub fn execute_cancellable<T>(
        &self,
        query: &str,
        query_args: &[&dyn ToSql],
        cancelled_hook: Option<&dyn Fn() -> bool>,
        mut mapper: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
    ) -> HydrusResult<Vec<T>> {
        if let Some(hook) = cancelled_hook
            && hook()
        {
            return Ok(Vec::new());
        }

        let connection = self.lock();

        Self::maybe_print_query_plan(&connection, query, query_args);

        let mut statement = connection
            .prepare(query)
            .map_err(|error| db_error(format!("Could not prepare \"{query}\""), error))?;

        let mut rows = statement
            .query(query_args)
            .map_err(|error| db_error(format!("Could not run \"{query}\""), error))?;

        let mut results = Vec::new();

        while let Some(row) = rows
            .next()
            .map_err(|error| db_error(format!("Could not read a row from \"{query}\""), error))?
        {
            results.push(
                mapper(row)
                    .map_err(|error| db_error(format!("Could not read a row from \"{query}\""), error))?,
            );

            if let Some(hook) = cancelled_hook
                && hook()
            {
                break;
            }
        }

        Ok(results)
    }

    /// Query one row, mapping it with `mapper`.
    pub fn query_one<T>(
        &self,
        query: &str,
        query_args: &[&dyn ToSql],
        mapper: impl FnOnce(&Row<'_>) -> rusqlite::Result<T>,
    ) -> HydrusResult<Option<T>> {
        let connection = self.lock();

        Self::maybe_print_query_plan(&connection, query, query_args);

        let mut statement = connection
            .prepare(query)
            .map_err(|error| db_error(format!("Could not prepare \"{query}\""), error))?;

        statement
            .query_row(query_args, mapper)
            .optional()
            .map_err(|error| db_error(format!("Could not run \"{query}\""), error))
    }

    /// Query every row, collecting into a `Vec`.
    pub fn query_all<T>(
        &self,
        query: &str,
        query_args: &[&dyn ToSql],
        mapper: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
    ) -> HydrusResult<Vec<T>> {
        let connection = self.lock();

        Self::maybe_print_query_plan(&connection, query, query_args);

        let mut statement = connection
            .prepare(query)
            .map_err(|error| db_error(format!("Could not prepare \"{query}\""), error))?;

        let rows = statement
            .query_map(query_args, mapper)
            .map_err(|error| db_error(format!("Could not run \"{query}\""), error))?;

        let mut results = Vec::new();

        for row in rows {
            results.push(row.map_err(|error| db_error(format!("Could not read a row from \"{query}\""), error))?);
        }

        Ok(results)
    }

    /// `_execute( 'ANALYZE ...' )` for a temp table, then refresh mem stats.
    pub fn analyze_temp_table(&self, temp_table_name: &str) -> HydrusResult<()> {
        let connection = self.lock();

        connection
            .execute_batch(&format!(
                "ANALYZE {temp_table_name}; ANALYZE mem.sqlite_master;"
            ))
            .map_err(|error| db_error("Could not analyse a temporary table", error))
    }

    /// Python closes its cursor; the shared connection stays open, so this is a
    /// no-op kept for API parity.
    pub fn close_cursor(&self) {}

    /// `_create_index`
    pub fn create_index(
        &self,
        table_name: &str,
        columns: &[&str],
        unique: bool,
    ) -> HydrusResult<()> {
        let create_phrase = if unique {
            "CREATE UNIQUE INDEX IF NOT EXISTS"
        } else {
            "CREATE INDEX IF NOT EXISTS"
        };

        let ideal_index_name = self.generate_ideal_index_name(table_name, columns);

        let mut index_name = ideal_index_name.clone();
        let mut i = 0usize;

        while self.actual_index_exists(&index_name)? {
            index_name = format!("{ideal_index_name}_{i}");
            i += 1;
        }

        let table_name_simple = table_name
            .split_once('.')
            .map(|(_, simple)| simple)
            .unwrap_or(table_name);

        let statement = format!(
            "{} {} ON {} ({});",
            create_phrase,
            index_name,
            table_name_simple,
            columns.join(", ")
        );

        self.execute(&statement, &[])?;

        Ok(())
    }

    /// `_generate_ideal_index_name`
    pub fn generate_ideal_index_name(&self, table_name: &str, columns: &[&str]) -> String {
        format!("{}_{}_index", table_name, columns.join("_"))
    }

    /// `_get_attached_database_names`
    pub fn get_attached_database_names(&self, include_temp: bool) -> HydrusResult<Vec<String>> {
        let rows = self.query_all("PRAGMA database_list;", &[], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        Ok(rows
            .into_iter()
            .filter(|(_, schema_name, path)| {
                if include_temp {
                    true
                } else {
                    schema_name != "temp" && !path.is_empty()
                }
            })
            .map(|(_, schema_name, _)| schema_name)
            .collect())
    }

    /// `_get_last_row_id`
    pub fn get_last_row_id(&self) -> i64 {
        self.lock().last_insert_rowid()
    }

    /// `_get_row_count`. A negative rusqlite count means "unknown", like -1 in
    /// Python.
    pub fn get_row_count(&self) -> usize {
        self.connection.lock().map(|connection| connection.changes()).unwrap_or(0) as usize
    }

    /// `_get_sum_result`
    pub fn get_sum_result(&self, result: Option<Option<i64>>) -> i64 {
        result.flatten().unwrap_or(0)
    }

    /// `_actual_index_exists`
    pub fn actual_index_exists(&self, index_name: &str) -> HydrusResult<bool> {
        let (search_schemas, index_name) = match index_name.split_once('.') {
            Some((schema, index_name)) => (vec![schema.to_string()], index_name),
            None => (self.get_attached_database_names(false)?, index_name),
        };

        for schema in search_schemas {
            let result = self.query_one(
                &format!(
                    "SELECT 1 FROM {schema}.sqlite_master WHERE name = ? and type = ?;"
                ),
                &[&index_name, &"index"],
                |row| row.get::<_, i64>(0),
            )?;

            if result.is_some() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// `_ideal_index_exists`
    pub fn ideal_index_exists(&self, table_name: &str, columns: &[&str]) -> HydrusResult<bool> {
        let ideal_index_name = self.generate_ideal_index_name(table_name, columns);

        let (search_schemas, ideal_index_name) = match ideal_index_name.split_once('.') {
            Some((schema, index_name)) => (vec![schema.to_string()], index_name.to_string()),
            None => (self.get_attached_database_names(false)?, ideal_index_name),
        };

        let table_name = table_name
            .split_once('.')
            .map(|(_, simple)| simple)
            .unwrap_or(table_name);

        for schema in search_schemas {
            let table_result = self.query_one(
                &format!("SELECT 1 FROM {schema}.sqlite_master WHERE name = ?;"),
                &[&table_name],
                |row| row.get::<_, i64>(0),
            )?;

            if table_result.is_none() {
                continue;
            }

            let index_names = self.query_all(
                &format!(
                    "SELECT name FROM {schema}.sqlite_master WHERE tbl_name = ? AND type = ?;"
                ),
                &[&table_name, &"index"],
                |row| row.get::<_, String>(0),
            )?;

            if index_names
                .iter()
                .any(|index_name| index_name.contains(&ideal_index_name))
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// `_make_temporary_integer_table` (single column)
    pub fn make_temporary_integer_table(
        &self,
        integers: &[i64],
        column_name: &str,
    ) -> HydrusResult<TemporaryIntegerTable> {
        TemporaryIntegerTable::new(self.connection.clone(), integers, column_name)
    }

    /// Multi-column variant of `_make_temporary_integer_table`.
    pub fn make_temporary_integer_table_with_rows(
        &self,
        rows: &[Vec<i64>],
        column_names: &[&str],
    ) -> HydrusResult<TemporaryIntegerTable> {
        TemporaryIntegerTable::with_rows(self.connection.clone(), rows, column_names)
    }

    /// `_set_cursor` is meaningless with a shared connection; this swaps the
    /// connection instead.
    pub fn set_connection(&mut self, connection: SharedConnection) {
        self.connection = connection;
    }

    /// `_table_exists`
    pub fn table_exists(&self, table_name: &str) -> HydrusResult<bool> {
        let (search_schemas, table_name) = match table_name.split_once('.') {
            Some((schema, table_name)) => (vec![schema.to_string()], table_name),
            None => (self.get_attached_database_names(false)?, table_name),
        };

        for schema in search_schemas {
            let result = self.query_one(
                &format!(
                    "SELECT 1 FROM {schema}.sqlite_master WHERE name = ? AND type = ?;"
                ),
                &[&table_name, &"table"],
                |row| row.get::<_, i64>(0),
            )?;

            if result.is_some() {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

/// Python's `DBCursorTransactionWrapper`.
pub struct DBCursorTransactionWrapper {
    db: DBBase,
    transaction_commit_period: i64,
    transaction_start_time: i64,
    in_transaction: bool,
    transaction_contains_writes: bool,
    committing_as_soon_as_possible: bool,
    last_mem_refresh_time: i64,
    last_wal_passive_checkpoint_time: i64,
    last_wal_truncate_checkpoint_time: i64,
    last_journal_zero_time: i64,
    pubsubs: Vec<PubSubMessage>,
}

impl DBCursorTransactionWrapper {
    pub fn new(connection: SharedConnection, transaction_commit_period: i64) -> Self {
        let now = hydrus_time::get_now();

        Self {
            db: DBBase::new(connection),
            transaction_commit_period,
            transaction_start_time: 0,
            in_transaction: false,
            transaction_contains_writes: false,
            committing_as_soon_as_possible: false,
            last_mem_refresh_time: now,
            last_wal_passive_checkpoint_time: now,
            last_wal_truncate_checkpoint_time: now,
            last_journal_zero_time: now,
            pubsubs: Vec::new(),
        }
    }

    pub fn db(&self) -> &DBBase {
        &self.db
    }

    fn zero_journal(&mut self) -> HydrusResult<()> {
        let journal_mode = hydrus_globals::db_journal_mode();

        if journal_mode != "PERSIST" && journal_mode != "WAL" {
            return Ok(());
        }

        self.db.execute("BEGIN IMMEDIATE;", &[])?;

        let db_names = self
            .db
            .query_all("PRAGMA database_list;", &[], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?
            .into_iter()
            .filter(|(_, name, _)| name != "mem" && name != "temp")
            .map(|(_, name, _)| name)
            .collect::<Vec<_>>();

        for db_name in &db_names {
            self.db
                .execute(&format!("PRAGMA {db_name}.journal_size_limit = 0;"), &[])?;
        }

        self.db.execute("COMMIT;", &[])?;

        for db_name in &db_names {
            self.db.execute(
                &format!("PRAGMA {db_name}.journal_size_limit = {JOURNAL_SIZE_LIMIT};"),
                &[],
            )?;
        }

        Ok(())
    }

    /// `begin_immediate`
    pub fn begin_immediate(&mut self) -> HydrusResult<()> {
        if !self.in_transaction {
            self.db.execute("BEGIN IMMEDIATE;", &[])?;
            self.db.execute("SAVEPOINT hydrus_savepoint;", &[])?;

            self.transaction_start_time = hydrus_time::get_now();
            self.in_transaction = true;
            self.transaction_contains_writes = false;
        }

        Ok(())
    }

    /// `clean_pub_subs`
    pub fn clean_pub_subs(&mut self) {
        self.pubsubs.clear();
    }

    /// `commit`
    pub fn commit(&mut self) -> HydrusResult<()> {
        if self.in_transaction {
            self.clean_pub_subs();

            self.db.execute("COMMIT;", &[])?;

            self.in_transaction = false;
            self.transaction_contains_writes = false;

            let journal_mode = hydrus_globals::db_journal_mode();

            if journal_mode == "WAL"
                && hydrus_time::time_has_passed(Some(
                    self.last_wal_passive_checkpoint_time + WAL_PASSIVE_CHECKPOINT_PERIOD,
                ))
            {
                if hydrus_time::time_has_passed(Some(
                    self.last_wal_truncate_checkpoint_time + WAL_TRUNCATE_CHECKPOINT_PERIOD,
                )) {
                    self.db.execute("PRAGMA wal_checkpoint(TRUNCATE);", &[])?;

                    self.last_wal_truncate_checkpoint_time = hydrus_time::get_now();
                } else {
                    self.db.execute("PRAGMA wal_checkpoint(PASSIVE);", &[])?;
                }

                self.last_wal_passive_checkpoint_time = hydrus_time::get_now();
            }

            if hydrus_time::time_has_passed(Some(
                self.last_mem_refresh_time + MEM_REFRESH_PERIOD,
            )) {
                self.db.execute("DETACH mem;", &[])?;
                self.db.execute("ATTACH \":memory:\" AS mem;", &[])?;

                TemporaryIntegerTableNameCache::instance().clear();

                self.last_mem_refresh_time = hydrus_time::get_now();
            }

            if journal_mode == "PERSIST"
                && hydrus_time::time_has_passed(Some(
                    self.last_journal_zero_time + JOURNAL_ZERO_PERIOD,
                ))
            {
                self.zero_journal()?;

                self.last_journal_zero_time = hydrus_time::get_now();
            }
        } else {
            hydrus_data::safe_print("Received a call to commit, but was not in a transaction!");
        }

        self.committing_as_soon_as_possible = false;

        Ok(())
    }

    /// `commit_and_begin`
    pub fn commit_and_begin(&mut self) -> HydrusResult<()> {
        if self.in_transaction {
            self.commit()?;
            self.begin_immediate()?;
        }

        Ok(())
    }

    /// `do_a_commit_as_soon_as_possible`
    pub fn do_a_commit_as_soon_as_possible(&mut self) {
        self.committing_as_soon_as_possible = true;
    }

    /// `do_pub_subs`. Python publishes to `HG.controller`; the port hands the
    /// queued messages back so the db owner can publish them.
    pub fn do_pub_subs(&mut self) -> Vec<PubSubMessage> {
        std::mem::take(&mut self.pubsubs)
    }

    /// `is_committing_as_soon_as_possible`
    pub fn is_committing_as_soon_as_possible(&self) -> bool {
        self.committing_as_soon_as_possible
    }

    /// `in_transaction`
    pub fn in_transaction(&self) -> bool {
        self.in_transaction
    }

    /// `notify_write_occuring`
    pub fn notify_write_occuring(&mut self) {
        self.transaction_contains_writes = true;
    }

    /// `pub_after_job( topic, *args, **kwargs )`
    pub fn pub_after_job(
        &mut self,
        topic: impl Into<String>,
        args: Vec<Value>,
        kwargs: Map<String, Value>,
    ) {
        let topic = topic.into();

        if args.is_empty() && kwargs.is_empty() {
            let already_queued = self.pubsubs.iter().any(|message| {
                message.topic == topic && message.args.is_empty() && message.kwargs.is_empty()
            });

            if already_queued {
                return;
            }
        }

        self.pubsubs.push(PubSubMessage {
            topic,
            args,
            kwargs,
        });
    }

    /// `rollback`
    pub fn rollback(&mut self) -> HydrusResult<()> {
        if self.in_transaction {
            self.db.execute("ROLLBACK TO hydrus_savepoint;", &[])?;

            TemporaryIntegerTableNameCache::instance().clear();
        } else {
            hydrus_data::safe_print("Received a call to rollback, but was not in a transaction!");
        }

        Ok(())
    }

    /// `save`
    pub fn save(&mut self) -> HydrusResult<()> {
        if self.in_transaction {
            if let Err(error) = self.db.execute("RELEASE hydrus_savepoint;", &[]) {
                hydrus_data::safe_print(format!(
                    "Tried to release a database savepoint, but failed! {error}"
                ));
            }

            self.db.execute("SAVEPOINT hydrus_savepoint;", &[])?;
        } else {
            hydrus_data::safe_print("Received a call to save, but was not in a transaction!");
        }

        Ok(())
    }

    /// `time_to_commit`
    pub fn time_to_commit(&self) -> bool {
        let p1 = self.committing_as_soon_as_possible;

        let p2 = self.transaction_contains_writes
            && hydrus_time::time_has_passed(Some(
                self.transaction_start_time + self.transaction_commit_period,
            ));

        self.in_transaction && (p1 || p2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The temporary-table name cache is process-global; these tests must not
    /// clear it while another test has a table open.
    static TEMP_TABLE_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn temp_db(name: &str) -> std::path::PathBuf {
        let dir = hydrus_temp::get_sub_temp_dir(&format!("db_base_{name}")).expect("temp dir");

        dir.join("test.db")
    }

    fn new_db(name: &str) -> (DBBase, std::path::PathBuf) {
        let path = temp_db(name);
        let db = DBBase::open(&path).expect("db");

        (db, path)
    }

    #[test]
    fn temporary_table_names_are_reused() {
        let _guard = TEMP_TABLE_TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());

        let cache = TemporaryIntegerTableNameCache::instance();

        cache.clear();

        let (initialised, first) = cache.get_name(&["hash_id"]);
        let (second_initialised, second) = cache.get_name(&["hash_id"]);

        // A name handed out is popped immediately, so an unreleased name is
        // never reused and the counter advances.
        assert!(!initialised);
        assert!(!second_initialised);
        assert_ne!(first, second);

        cache.release_name(&["hash_id"], &first);

        let (third_initialised, third) = cache.get_name(&["hash_id"]);

        assert!(third_initialised);
        assert_eq!(third, first);

        cache.clear();
    }

    #[test]
    fn temporary_integer_table_round_trip() {
        let _guard = TEMP_TABLE_TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());

        let (db, path) = new_db("temp_table");

        db.execute("ATTACH \":memory:\" AS mem;", &[]).expect("attach");

        let table_name;

        {
            let table = db
                .make_temporary_integer_table(&[3, 1, 2, 2], "hash_id")
                .expect("table");

            // The cache is process-global and other tests use the same column
            // name, so only the shape can be asserted.
            assert!(table.table_name().starts_with("mem.temp_1_int_hash_id_"));

            table_name = table.table_name().to_string();

            let values = db
                .query_all(
                    &format!(
                        "SELECT hash_id FROM {} ORDER BY hash_id;",
                        table.table_name()
                    ),
                    &[],
                    |row| row.get::<_, i64>(0),
                )
                .expect("values");

            assert_eq!(values, vec![1, 2, 3]);
        }

        let count = db
            .query_one(
                &format!("SELECT COUNT( * ) FROM {table_name};"),
                &[],
                |row| row.get::<_, i64>(0),
            )
            .expect("count");

        assert_eq!(count, Some(0));

        drop(std::fs::remove_file(&path));
        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }

    #[test]
    fn table_index_and_attachment_helpers() {
        let (db, path) = new_db("helpers");

        db.execute(
            "CREATE TABLE things ( thing_id INTEGER PRIMARY KEY, name TEXT );",
            &[],
        )
        .expect("table");

        assert!(db.table_exists("things").expect("exists"));
        assert!(!db.table_exists("missing").expect("exists"));

        db.create_index("things", &["name"], false).expect("index");

        assert!(
            db.actual_index_exists("things_name_index")
                .expect("actual index")
        );
        assert!(
            db.ideal_index_exists("things", &["name"])
                .expect("ideal index")
        );

        db.create_index("things", &["name"], false)
            .expect("second index");

        assert!(
            db.actual_index_exists("things_name_index_0")
                .expect("actual index")
        );

        assert!(db.get_attached_database_names(false).expect("names").contains(&"main".to_string()));

        drop(std::fs::remove_file(&path));
        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }

    #[test]
    fn transaction_wrapper_commits_rolls_back_and_saves() {
        let (db, path) = new_db("transactions");
        let connection = db.connection();

        db.execute("CREATE TABLE numbers ( value INTEGER );", &[])
            .expect("table");

        let mut wrapper = DBCursorTransactionWrapper::new(connection, 30);

        assert!(!wrapper.in_transaction());
        assert!(!wrapper.time_to_commit());

        wrapper.begin_immediate().expect("begin");

        assert!(wrapper.in_transaction());

        wrapper.notify_write_occuring();

        assert!(!wrapper.time_to_commit());

        wrapper.do_a_commit_as_soon_as_possible();

        assert!(wrapper.time_to_commit());

        db.execute("INSERT INTO numbers VALUES ( 1 );", &[])
            .expect("insert");
        wrapper.save().expect("save");

        db.execute("INSERT INTO numbers VALUES ( 2 );", &[])
            .expect("insert");
        wrapper.rollback().expect("rollback");

        db.execute("INSERT INTO numbers VALUES ( 3 );", &[])
            .expect("insert");
        wrapper.commit().expect("commit");

        let values = db
            .query_all("SELECT value FROM numbers ORDER BY value;", &[], |row| {
                row.get::<_, i64>(0)
            })
            .expect("values");

        // `save` releases the savepoint and immediately creates a new one, so
        // the rollback undoes only the insert that followed it.
        assert_eq!(values, vec![1, 3]);

        drop(std::fs::remove_file(&path));
        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }

    #[test]
    fn pubsubs_are_queued_once() {
        let (db, path) = new_db("pubsubs");
        let mut wrapper = DBCursorTransactionWrapper::new(db.connection(), 30);

        wrapper.pub_after_job("update", vec![], Map::new());
        wrapper.pub_after_job("update", vec![], Map::new());

        let mut with_args = Map::new();
        with_args.insert("value".to_string(), Value::from(1));

        wrapper.pub_after_job("update", vec![], with_args.clone());
        wrapper.pub_after_job("update", vec![], with_args);

        // Only the no-argument form is de-duplicated, exactly like Python.
        let messages = wrapper.do_pub_subs();

        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].topic, "update");
        assert!(wrapper.do_pub_subs().is_empty());

        drop(std::fs::remove_file(&path));
        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }

    #[test]
    fn job_database_delivers_results_and_errors() {
        let job = JobDatabase::new("read", true, "get_number", || {
            Ok(Box::new(42i64) as Box<dyn std::any::Any + Send>)
        });

        assert_eq!(job.get_type(), "read");
        assert!(job.is_synchronous());
        assert_eq!(job.to_string(), "read get_number");

        let action = job.take_action().expect("action");

        job.put_result(action());

        let result = job.get_result().expect("result");

        assert_eq!(result.downcast_ref::<i64>(), Some(&42));

        let failing = JobDatabase::new("write", false, "fail", || {
            Err(HydrusException::new(ErrorKind::Db, "nope"))
        });

        let action = failing.take_action().expect("action");

        failing.put_result(action());

        let error = failing.get_result().expect_err("error");

        assert!(error.is_a(ErrorKind::Db));
    }

    #[test]
    fn cancellable_reads_stop_early() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let (db, path) = new_db("cancellable");

        db.execute("CREATE TABLE numbers ( value INTEGER );", &[])
            .expect("table");

        for value in 0..10 {
            db.execute("INSERT INTO numbers VALUES ( ? );", &[&value])
                .expect("insert");
        }

        let cancelled = AtomicBool::new(false);

        let values = db
            .execute_cancellable(
                "SELECT value FROM numbers ORDER BY value;",
                &[],
                Some(&|| cancelled.load(Ordering::Relaxed)),
                |row| row.get::<_, i64>(0),
            )
            .expect("values");

        assert_eq!(values.len(), 10);

        cancelled.store(true, Ordering::Relaxed);

        let values = db
            .execute_cancellable(
                "SELECT value FROM numbers;",
                &[],
                Some(&|| cancelled.load(Ordering::Relaxed)),
                |row| row.get::<_, i64>(0),
            )
            .expect("values");

        assert!(values.is_empty());

        drop(std::fs::remove_file(&path));
        drop(std::fs::remove_dir_all(path.parent().unwrap()));
    }

    #[test]
    fn cancellable_cursor_stops_on_hook() {
        let values = read_from_cancellable_cursor(0..100, 8, Some(&|| false));

        assert_eq!(values.len(), 100);

        let values = read_from_cancellable_cursor(0..100, 8, Some(&|| true));

        assert!(values.is_empty());
    }
}
