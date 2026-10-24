//! Client DB module wrapper around `hydrus_core::DBModule`.

use std::collections::BTreeSet;
use hydrus_core::hydrus_db_base::DBBase;
use hydrus_core::hydrus_db_module::{
    DBModule, IndexGenerationDict, TableGenerationDict, TableGenerationRow,
};

/// A declarative client DB module. Individual client modules expose their
/// table/index generation dictionaries; repair and creation are handled by the
/// core trait exactly as on the server.
pub struct ClientDBModule<'a> {
    name: &'a str,
    db: &'a DBBase,
    initial_tables: TableGenerationDict,
    initial_indices: IndexGenerationDict,
    critical_tables: BTreeSet<String>,
    service_prefixes: BTreeSet<String>,
    service_ids: Vec<i64>,
}

impl<'a> ClientDBModule<'a> {
    pub fn new(name: &'a str, db: &'a DBBase) -> Self {
        Self {
            name,
            db,
            initial_tables: TableGenerationDict::new(),
            initial_indices: IndexGenerationDict::new(),
            critical_tables: BTreeSet::new(),
            service_prefixes: BTreeSet::new(),
            service_ids: Vec::new(),
        }
    }

    pub fn with_table(
        mut self,
        table_name: impl Into<String>,
        create_query_without_name: impl Into<String>,
        version_added: i64,
    ) -> Self {
        self.initial_tables.insert(
            table_name.into(),
            (create_query_without_name.into(), version_added),
        );
        self
    }

    pub fn with_index(
        mut self,
        table_name: impl Into<String>,
        columns: Vec<String>,
        unique: bool,
        version_added: i64,
    ) -> Self {
        self.initial_indices
            .entry(table_name.into())
            .or_default()
            .push((columns, unique, version_added));
        self
    }

    pub fn with_critical_table(mut self, table_name: impl Into<String>) -> Self {
        self.critical_tables.insert(table_name.into());
        self
    }

    pub fn with_service_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.service_prefixes.insert(prefix.into());
        self
    }

    pub fn with_service_ids(mut self, service_ids: Vec<i64>) -> Self {
        self.service_ids = service_ids;
        self
    }

    pub fn table_rows(&self) -> &TableGenerationDict {
        &self.initial_tables
    }
}

impl DBModule for ClientDBModule<'_> {
    fn name(&self) -> &str {
        self.name
    }

    fn db(&self) -> &DBBase {
        self.db
    }

    fn get_initial_table_generation_dict(&self) -> TableGenerationDict {
        self.initial_tables.clone()
    }

    fn get_initial_index_generation_dict(&self) -> IndexGenerationDict {
        self.initial_indices.clone()
    }

    fn get_critical_table_names(&self) -> BTreeSet<String> {
        self.critical_tables.clone()
    }

    fn get_service_table_prefixes(&self) -> BTreeSet<String> {
        self.service_prefixes.clone()
    }

    fn get_service_ids_we_generate_dynamic_tables_for(&self) -> Vec<i64> {
        self.service_ids.clone()
    }
}

/// The tables the client itself owns (as opposed to master/definitions).
pub fn client_schema_module<'a>(db: &'a DBBase) -> ClientDBModule<'a> {
    let mut module = ClientDBModule::new("client schema", db);

    for (name, query) in [
        (
            "version",
            "CREATE TABLE IF NOT EXISTS {} ( version INTEGER );",
        ),
        (
            "options",
            "CREATE TABLE IF NOT EXISTS {} ( options TEXT );",
        ),
    ] {
        module = module.with_table(name, query, 1);
    }

    module
        .with_table(
            "services",
            "CREATE TABLE IF NOT EXISTS {} ( service_id INTEGER PRIMARY KEY, service_key BLOB_BYTES, service_type INTEGER, name TEXT, dictionary_string TEXT );",
            1,
        )
        .with_index("services", vec!["service_key".to_string()], true, 1)
        .with_critical_table("services")
        .with_critical_table("version")
}

#[cfg(test)]
mod tests {
    use super::*;
    use hydrus_core::hydrus_db_module::DBModule;

    fn database() -> (DBBase, std::path::PathBuf) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);

        let directory = std::env::temp_dir().join(format!(
            "hydrus_client_dbmodule_{}_{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&directory).unwrap();

        (DBBase::open(directory.join("client.db")).unwrap(), directory)
    }

    #[test]
    fn module_describes_and_creates_tables() {
        let (db, directory) = database();
        let module = client_schema_module(&db);

        assert_eq!(module.name(), "client schema");
        assert!(module.get_initial_table_generation_dict().contains_key("services"));
        assert!(module.get_critical_table_names().contains("version"));

        module.create_initial_tables().unwrap();
        module.create_initial_indices().unwrap();

        assert!(db.table_exists("services").unwrap());
        assert!(db.table_exists("version").unwrap());

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn table_rows_are_declarative() {
        let (db, directory) = database();
        let module = ClientDBModule::new("test", &db)
            .with_table("t", "CREATE TABLE IF NOT EXISTS {} ( x INTEGER );", 5);

        let row: &TableGenerationRow = module.table_rows().get("t").unwrap();

        assert_eq!(row.1, 5);

        let _ = std::fs::remove_dir_all(directory);
    }
}
