//! Rust port of `hydrus/core/HydrusDBModule.py`.
//!
//! Python's `HydrusDBModule` is an abstract base class whose subclasses supply
//! table/index generation dictionaries. The port makes it a trait: implementors
//! return their generation dictionaries and a [`DBBase`] handle, and get the
//! schema creation/repair helpers for free.
//!
//! The repair routine preserves Python's order: initial tables, their indices,
//! then per-service tables and indices, committing between each step.

use std::collections::{BTreeSet, HashMap};

use crate::hydrus_db_base::{DBBase, DBCursorTransactionWrapper};
use crate::hydrus_exceptions::{ErrorKind, HydrusException, HydrusResult};

/// `(columns, unique, version_added)`
pub type IndexGenerationRow = (Vec<String>, bool, i64);

/// `(create_query_without_name, version_added)`
pub type TableGenerationRow = (String, i64);

/// `{ table_name: [ ( columns, unique, version_added ), ... ] }`
pub type IndexGenerationDict = HashMap<String, Vec<IndexGenerationRow>>;

/// `{ table_name: ( create_query_without_name, version_added ) }`
pub type TableGenerationDict = HashMap<String, TableGenerationRow>;

fn not_implemented(what: &str) -> HydrusException {
    HydrusException::new(
        ErrorKind::Unknown,
        format!("{what} is not implemented for this db module!"),
    )
}

/// Python's `HydrusDBModule`.
pub trait DBModule {
    /// `self.name`
    fn name(&self) -> &str;

    /// The shared connection helpers this module runs against.
    fn db(&self) -> &DBBase;

    /// `CAN_REPOPULATE_ALL_MISSING_DATA`
    fn can_repopulate_all_missing_data(&self) -> bool {
        false
    }

    /// `_get_initial_table_generation_dict`
    fn get_initial_table_generation_dict(&self) -> TableGenerationDict {
        TableGenerationDict::new()
    }

    /// `_get_initial_index_generation_dict`
    fn get_initial_index_generation_dict(&self) -> IndexGenerationDict {
        IndexGenerationDict::new()
    }

    /// `_get_service_table_generation_dict( service_id )`
    fn get_service_table_generation_dict(&self, _service_id: i64) -> TableGenerationDict {
        TableGenerationDict::new()
    }

    /// `_get_service_index_generation_dict( service_id )`
    fn get_service_index_generation_dict(&self, _service_id: i64) -> IndexGenerationDict {
        IndexGenerationDict::new()
    }

    /// `_get_critical_table_names`
    fn get_critical_table_names(&self) -> BTreeSet<String> {
        BTreeSet::new()
    }

    /// `_get_service_table_prefixes`
    fn get_service_table_prefixes(&self) -> BTreeSet<String> {
        BTreeSet::new()
    }

    /// `_get_service_ids_we_generate_dynamic_tables_for`
    fn get_service_ids_we_generate_dynamic_tables_for(&self) -> Vec<i64> {
        Vec::new()
    }

    /// `_do_last_shutdown_was_bad_work`
    fn do_last_shutdown_was_bad_work_internal(&self) {}

    /// `_repair_repopulate_tables`
    fn repair_repopulate_tables(
        &self,
        _table_names: &[String],
        _cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
    ) {
    }

    /// `_present_missing_tables_warning_to_user`
    fn present_missing_tables_warning_to_user(&self, _table_names: &[String]) -> HydrusResult<()> {
        Err(not_implemented("present_missing_tables_warning_to_user"))
    }

    /// `_present_missing_indices_warning_to_user`
    fn present_missing_indices_warning_to_user(
        &self,
        _index_names: &[String],
    ) -> HydrusResult<()> {
        Err(not_implemented(
            "present_missing_indices_warning_to_user",
        ))
    }

    /// `get_tables_and_columns_that_use_definitions`
    fn get_tables_and_columns_that_use_definitions(
        &self,
        _content_type: i64,
    ) -> HydrusResult<Vec<(String, String)>> {
        Err(not_implemented(
            "get_tables_and_columns_that_use_definitions",
        ))
    }

    /// `_flatten_index_generation_dict`
    fn flatten_index_generation_dict(
        &self,
        index_generation_dict: &IndexGenerationDict,
    ) -> Vec<(String, Vec<String>, bool, i64)> {
        let mut tuples = Vec::new();

        for (table_name, index_rows) in index_generation_dict {
            for (columns, unique, version_added) in index_rows {
                tuples.push((
                    table_name.clone(),
                    columns.clone(),
                    *unique,
                    *version_added,
                ));
            }
        }

        tuples
    }

    /// `_create_table`
    fn create_table(&self, create_query_without_name: &str, table_name: &str) -> HydrusResult<()> {
        if create_query_without_name.to_lowercase().contains("fts4(") {
            // When repairing a missing fts4 table, damaged old virtual tables
            // can leave their subtables behind and break the new CREATE.
            let (raw_table_name, sqlite_master_table) = match table_name.split_once('.') {
                Some((schema, raw_table_name)) => (
                    raw_table_name.to_string(),
                    format!("{schema}.sqlite_master"),
                ),
                None => (table_name.to_string(), "sqlite_master".to_string()),
            };

            let exists = self.db().query_one(
                &format!("SELECT 1 FROM {sqlite_master_table} WHERE name = ?;"),
                &[&raw_table_name],
                |row| row.get::<_, i64>(0),
            )?;

            if exists.is_none() {
                let possible_suffixes = [
                    "_content",
                    "_docsize",
                    "_segdir",
                    "_segments",
                    "_stat",
                ];

                for suffix in possible_suffixes {
                    let possible_subtable_name = format!("{raw_table_name}{suffix}");

                    let subtable_exists = self.db().query_one(
                        &format!("SELECT 1 FROM {sqlite_master_table} WHERE name = ?;"),
                        &[&possible_subtable_name],
                        |row| row.get::<_, i64>(0),
                    )?;

                    if subtable_exists.is_some() {
                        self.db()
                            .execute(&format!("DROP TABLE {possible_subtable_name};"), &[])?;
                    }
                }
            }
        }

        self.db().execute(
            &create_query_without_name.replace("{}", table_name),
            &[],
        )?;

        Ok(())
    }

    /// `create_initial_indices`
    fn create_initial_indices(&self) -> HydrusResult<()> {
        let index_generation_dict = self.get_initial_index_generation_dict();

        for (table_name, columns, unique, _version_added) in
            self.flatten_index_generation_dict(&index_generation_dict)
        {
            let columns = columns.iter().map(String::as_str).collect::<Vec<_>>();

            self.db().create_index(&table_name, &columns, unique)?;
        }

        Ok(())
    }

    /// `create_initial_tables`
    fn create_initial_tables(&self) -> HydrusResult<()> {
        let table_generation_dict = self.get_initial_table_generation_dict();

        for (table_name, (create_query_without_name, _version_added)) in table_generation_dict {
            self.create_table(&create_query_without_name, &table_name)?;
        }

        Ok(())
    }

    /// `do_last_shutdown_was_bad_work`
    fn do_last_shutdown_was_bad_work(&self) {
        self.do_last_shutdown_was_bad_work_internal();
    }

    /// `_get_services_table_generation_dict`
    fn get_services_table_generation_dict(&self) -> TableGenerationDict {
        let mut table_generation_dict = TableGenerationDict::new();

        for service_id in self.get_service_ids_we_generate_dynamic_tables_for() {
            table_generation_dict.extend(self.get_service_table_generation_dict(service_id));
        }

        table_generation_dict
    }

    /// `_get_services_index_generation_dict`
    fn get_services_index_generation_dict(&self) -> IndexGenerationDict {
        let mut index_generation_dict = IndexGenerationDict::new();

        for service_id in self.get_service_ids_we_generate_dynamic_tables_for() {
            for (table_name, rows) in self.get_service_index_generation_dict(service_id) {
                index_generation_dict
                    .entry(table_name)
                    .or_default()
                    .extend(rows);
            }
        }

        index_generation_dict
    }

    /// `get_expected_service_table_names`
    fn get_expected_service_table_names(&self) -> Vec<String> {
        self.get_services_table_generation_dict()
            .into_keys()
            .collect()
    }

    /// `get_expected_initial_table_names`
    fn get_expected_initial_table_names(&self) -> Vec<String> {
        self.get_initial_table_generation_dict()
            .into_keys()
            .collect()
    }

    /// `get_surplus_service_table_names`
    fn get_surplus_service_table_names(&self, all_table_names: &[String]) -> BTreeSet<String> {
        let prefixes = self.get_service_table_prefixes();

        if prefixes.is_empty() {
            return BTreeSet::new();
        }

        // Flatten to bare table names so schema qualification cannot produce a
        // false positive.
        let all_service_table_names = all_table_names
            .iter()
            .map(|name| name.split_once('.').map(|(_, bare)| bare).unwrap_or(name))
            .filter(|name| prefixes.iter().any(|prefix| name.starts_with(prefix)))
            .map(str::to_string)
            .collect::<BTreeSet<_>>();

        let good_service_table_names = self
            .get_expected_service_table_names()
            .into_iter()
            .map(|name| {
                name.split_once('.')
                    .map(|(_, bare)| bare.to_string())
                    .unwrap_or(name)
            })
            .collect::<BTreeSet<_>>();

        all_service_table_names
            .difference(&good_service_table_names)
            .cloned()
            .collect()
    }

    /// `repair( current_db_version, cursor_transaction_wrapper )`
    fn repair(
        &self,
        current_db_version: i64,
        cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
    ) -> HydrusResult<()> {
        self.repair_initial_tables(current_db_version, cursor_transaction_wrapper)?;
        self.repair_initial_indices(current_db_version, cursor_transaction_wrapper)?;
        self.repair_service_tables(current_db_version, cursor_transaction_wrapper)?;
        self.repair_service_indices(current_db_version, cursor_transaction_wrapper)?;

        Ok(())
    }

    fn repair_initial_tables(
        &self,
        current_db_version: i64,
        cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
    ) -> HydrusResult<()> {
        let table_generation_dict = self.get_initial_table_generation_dict();

        let missing_table_rows = table_generation_dict
            .iter()
            .filter(|(table_name, (_query, version_added))| {
                *version_added <= current_db_version && !self.db().table_exists(table_name).unwrap_or(true)
            })
            .map(|(table_name, (query, _))| (table_name.clone(), query.clone()))
            .collect::<Vec<_>>();

        if missing_table_rows.is_empty() {
            return Ok(());
        }

        let missing_table_names = {
            let mut names = missing_table_rows
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>();

            names.sort();

            names
        };

        let critical_table_names = self.get_critical_table_names();

        let missing_critical_table_names = missing_table_names
            .iter()
            .filter(|name| critical_table_names.contains(*name))
            .cloned()
            .collect::<BTreeSet<_>>();

        if !missing_critical_table_names.is_empty() {
            let mut message = "Unfortunately, this database is missing one or more critical tables! This database is non functional and cannot be repaired.".to_string();

            message += "\n\n";
            message += "If you have used this database many times before, please check the 'Recovery->Help my db is broke' document in the help for the next steps.";
            message += "\n\n";
            message += "If you are trying to initialise a new database and have yet to boot into it successfully, then the database files are damaged stubs. Go into your db dir (probably install_dir/db) and delete the small .db files and any .db-shm or .db-wal files with them, and then try booting the program again. If the problem keeps happening, you probably have a hard drive permission error to fix.";
            message += "\n\n";
            message += "The critical missing table names are:\n\n";
            message += &missing_critical_table_names
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");

            return Err(HydrusException::new(ErrorKind::DbAccess, message));
        }

        self.present_missing_tables_warning_to_user(&missing_table_names)?;

        for (table_name, create_query_without_name) in &missing_table_rows {
            self.create_table(create_query_without_name, table_name)?;

            cursor_transaction_wrapper.commit_and_begin()?;
        }

        self.repair_repopulate_tables(&missing_table_names, cursor_transaction_wrapper);

        cursor_transaction_wrapper.commit_and_begin()?;

        Ok(())
    }

    fn repair_initial_indices(
        &self,
        current_db_version: i64,
        cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
    ) -> HydrusResult<()> {
        let index_generation_dict = self.get_initial_index_generation_dict();

        let missing_index_rows = self
            .flatten_index_generation_dict(&index_generation_dict)
            .into_iter()
            .filter(|(table_name, columns, _unique, version_added)| {
                *version_added <= current_db_version
                    && !self
                        .db()
                        .ideal_index_exists(
                            table_name,
                            &columns.iter().map(String::as_str).collect::<Vec<_>>(),
                        )
                        .unwrap_or(true)
            })
            .collect::<Vec<_>>();

        if missing_index_rows.is_empty() {
            return Ok(());
        }

        let mut index_names = missing_index_rows
            .iter()
            .map(|(table_name, columns, _unique, _version_added)| {
                self.db().generate_ideal_index_name(
                    table_name,
                    &columns.iter().map(String::as_str).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        index_names.sort();

        self.present_missing_indices_warning_to_user(&index_names)?;

        for (table_name, columns, unique, _version_added) in missing_index_rows {
            let columns = columns.iter().map(String::as_str).collect::<Vec<_>>();

            self.db().create_index(&table_name, &columns, unique)?;

            cursor_transaction_wrapper.commit_and_begin()?;
        }

        Ok(())
    }

    fn repair_service_tables(
        &self,
        current_db_version: i64,
        cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
    ) -> HydrusResult<()> {
        let table_generation_dict = self.get_services_table_generation_dict();

        let missing_table_rows = table_generation_dict
            .iter()
            .filter(|(table_name, (_query, version_added))| {
                *version_added <= current_db_version && !self.db().table_exists(table_name).unwrap_or(true)
            })
            .map(|(table_name, (query, _))| (table_name.clone(), query.clone()))
            .collect::<Vec<_>>();

        if missing_table_rows.is_empty() {
            return Ok(());
        }

        let missing_table_names = {
            let mut names = missing_table_rows
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>();

            names.sort();

            names
        };

        self.present_missing_tables_warning_to_user(&missing_table_names)?;

        for (table_name, create_query_without_name) in &missing_table_rows {
            self.create_table(create_query_without_name, table_name)?;

            cursor_transaction_wrapper.commit_and_begin()?;
        }

        self.repair_repopulate_tables(&missing_table_names, cursor_transaction_wrapper);

        cursor_transaction_wrapper.commit_and_begin()?;

        Ok(())
    }

    fn repair_service_indices(
        &self,
        current_db_version: i64,
        cursor_transaction_wrapper: &mut DBCursorTransactionWrapper,
    ) -> HydrusResult<()> {
        let index_generation_dict = self.get_services_index_generation_dict();

        let missing_index_rows = self
            .flatten_index_generation_dict(&index_generation_dict)
            .into_iter()
            .filter(|(table_name, columns, _unique, version_added)| {
                *version_added <= current_db_version
                    && !self
                        .db()
                        .ideal_index_exists(
                            table_name,
                            &columns.iter().map(String::as_str).collect::<Vec<_>>(),
                        )
                        .unwrap_or(true)
            })
            .collect::<Vec<_>>();

        if missing_index_rows.is_empty() {
            return Ok(());
        }

        let mut index_names = missing_index_rows
            .iter()
            .map(|(table_name, columns, _unique, _version_added)| {
                self.db().generate_ideal_index_name(
                    table_name,
                    &columns.iter().map(String::as_str).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();

        index_names.sort();

        self.present_missing_indices_warning_to_user(&index_names)?;

        for (table_name, columns, unique, _version_added) in missing_index_rows {
            let columns = columns.iter().map(String::as_str).collect::<Vec<_>>();

            self.db().create_index(&table_name, &columns, unique)?;

            cursor_transaction_wrapper.commit_and_begin()?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use crate::hydrus_temp;

    struct TestModule {
        db: DBBase,
        name: String,
        warned_tables: Mutex<Vec<Vec<String>>>,
    }

    impl TestModule {
        fn new(db: DBBase) -> Self {
            Self {
                db,
                name: "test".to_string(),
                warned_tables: Mutex::new(Vec::new()),
            }
        }
    }

    impl DBModule for TestModule {
        fn name(&self) -> &str {
            &self.name
        }

        fn db(&self) -> &DBBase {
            &self.db
        }

        fn get_initial_table_generation_dict(&self) -> TableGenerationDict {
            TableGenerationDict::from([
                (
                    "things".to_string(),
                    (
                        "CREATE TABLE IF NOT EXISTS things ( thing_id INTEGER PRIMARY KEY, name TEXT );".to_string(),
                        10,
                    ),
                ),
                (
                    "future".to_string(),
                    (
                        "CREATE TABLE IF NOT EXISTS future ( id INTEGER );".to_string(),
                        100,
                    ),
                ),
            ])
        }

        fn get_initial_index_generation_dict(&self) -> IndexGenerationDict {
            IndexGenerationDict::from([(
                "things".to_string(),
                vec![(vec!["name".to_string()], false, 10)],
            )])
        }

        fn present_missing_tables_warning_to_user(
            &self,
            table_names: &[String],
        ) -> HydrusResult<()> {
            self.warned_tables
                .lock()
                .unwrap()
                .push(table_names.to_vec());

            Ok(())
        }

        fn present_missing_indices_warning_to_user(
            &self,
            _index_names: &[String],
        ) -> HydrusResult<()> {
            Ok(())
        }
    }

    #[test]
    fn creates_tables_indices_and_reports_expected_names() {
        let dir = hydrus_temp::get_sub_temp_dir("db_module").expect("temp dir");
        let path = dir.join("test.db");
        let db = DBBase::open(&path).expect("db");
        let module = TestModule::new(db.clone());

        module.create_initial_tables().expect("tables");
        module.create_initial_indices().expect("indices");

        assert!(db.table_exists("things").expect("table"));
        assert!(db.table_exists("future").expect("table"));
        assert!(db.ideal_index_exists("things", &["name"]).expect("index"));

        assert_eq!(module.get_expected_initial_table_names().len(), 2);
        assert_eq!(module.name(), "test");
        assert!(!module.can_repopulate_all_missing_data());

        drop(db);
        drop(std::fs::remove_dir_all(&dir));
    }

    #[test]
    fn repair_creates_missing_tables_and_indices() {
        let dir = hydrus_temp::get_sub_temp_dir("db_module_repair").expect("temp dir");
        let path = dir.join("test.db");
        let db = DBBase::open(&path).expect("db");
        let module = TestModule::new(db.clone());

        db.execute("CREATE TABLE version ( version INTEGER );", &[])
            .expect("version");

        let mut wrapper = DBCursorTransactionWrapper::new(db.connection(), 30);

        module.repair(10, &mut wrapper).expect("repair");

        assert!(db.table_exists("things").expect("table"));
        assert!(db.ideal_index_exists("things", &["name"]).expect("index"));
        assert!(!db.table_exists("future").expect("future table skipped"));

        let warnings = module.warned_tables.lock().unwrap();

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0], vec!["things".to_string()]);

        drop(warnings);
        drop(wrapper);
        drop(db);
        drop(std::fs::remove_dir_all(&dir));
    }

    #[test]
    fn missing_critical_tables_are_fatal() {
        struct CriticalModule {
            db: DBBase,
        }

        impl DBModule for CriticalModule {
            fn name(&self) -> &str {
                "critical"
            }

            fn db(&self) -> &DBBase {
                &self.db
            }

            fn get_initial_table_generation_dict(&self) -> TableGenerationDict {
                TableGenerationDict::from([(
                    "critical".to_string(),
                    (
                        "CREATE TABLE IF NOT EXISTS critical ( id INTEGER );".to_string(),
                        1,
                    ),
                )])
            }

            fn get_critical_table_names(&self) -> BTreeSet<String> {
                BTreeSet::from(["critical".to_string()])
            }
        }

        let dir = hydrus_temp::get_sub_temp_dir("db_module_critical").expect("temp dir");
        let path = dir.join("test.db");
        let db = DBBase::open(&path).expect("db");
        let module = CriticalModule { db: db.clone() };

        let mut wrapper = DBCursorTransactionWrapper::new(db.connection(), 30);

        let error = module.repair(1, &mut wrapper).expect_err("fatal");

        assert!(error.is_a(ErrorKind::DbAccess));
        assert!(error.to_string().contains("critical tables"));

        drop(wrapper);
        drop(db);
        drop(std::fs::remove_dir_all(&dir));
    }

    #[test]
    fn surplus_service_tables_are_found() {
        struct ServiceModule {
            db: DBBase,
        }

        impl DBModule for ServiceModule {
            fn name(&self) -> &str {
                "service"
            }

            fn db(&self) -> &DBBase {
                &self.db
            }

            fn get_service_table_prefixes(&self) -> BTreeSet<String> {
                BTreeSet::from(["files_".to_string()])
            }

            fn get_service_ids_we_generate_dynamic_tables_for(&self) -> Vec<i64> {
                vec![1]
            }

            fn get_service_table_generation_dict(&self, service_id: i64) -> TableGenerationDict {
                TableGenerationDict::from([(
                    format!("files_{service_id}"),
                    (
                        format!("CREATE TABLE IF NOT EXISTS files_{service_id} ( id INTEGER );"),
                        1,
                    ),
                )])
            }
        }

        let dir = hydrus_temp::get_sub_temp_dir("db_module_surplus").expect("temp dir");
        let path = dir.join("test.db");
        let db = DBBase::open(&path).expect("db");
        let module = ServiceModule { db: db.clone() };

        let all_table_names = vec![
            "main.files_1".to_string(),
            "files_2".to_string(),
            "other".to_string(),
        ];

        assert_eq!(
            module.get_surplus_service_table_names(&all_table_names),
            BTreeSet::from(["files_2".to_string()])
        );

        assert_eq!(
            module.get_expected_service_table_names(),
            vec!["files_1".to_string()]
        );

        drop(db);
        drop(std::fs::remove_dir_all(&dir));
    }
}
