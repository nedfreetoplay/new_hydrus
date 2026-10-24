impl ServerFiles {
    pub fn new(files_dir: impl Into<PathBuf>) -> Self {
        Self {
            files_dir: files_dir.into(),
        }
    }

    /// Resolve the files directory from `SG.server_controller`.
    pub fn from_global() -> HydrusResult<Self> {
        let controller = server_globals::require_server_controller()?;

        Ok(Self::new(controller.get_files_dir().to_path_buf()))
    }

    pub fn files_dir(&self) -> &Path {
        &self.files_dir
    }

    /// The directory setup from `DB._create_db`: the root plus every two-digit
    /// prefix folder.
    pub fn prepare(&self) -> HydrusResult<()> {
        hydrus_core::hydrus_paths::make_sure_directory_exists(&self.files_dir)?;

        for prefix in hydrus_files_physical_storage::iterate_prefixes("", 2) {
            hydrus_core::hydrus_paths::make_sure_directory_exists(
                &self.files_dir.join(prefix),
            )?;
        }

        Ok(())
    }

    /// `get_expected_file_path( hash )`
    pub fn get_expected_file_path(&self, hash: &[u8]) -> PathBuf {
        let hash_encoded = hex_encode(hash);
        let first_two_chars = &hash_encoded[..hash_encoded.len().min(2)];

        self.files_dir.join(first_two_chars).join(hash_encoded)
    }

    /// `get_expected_thumbnail_path( hash )`
    pub fn get_expected_thumbnail_path(&self, hash: &[u8]) -> PathBuf {
        let hash_encoded = hex_encode(hash);
        let first_two_chars = &hash_encoded[..hash_encoded.len().min(2)];

        self.files_dir
            .join(first_two_chars)
            .join(format!("{hash_encoded}.thumbnail"))
    }

    /// `get_file_path( hash )`
    pub fn get_file_path(&self, hash: &[u8]) -> HydrusResult<PathBuf> {
        let path = self.get_expected_file_path(hash);

        if !path.exists() {
            return Err(HydrusException::new(ErrorKind::NotFound, "File not found!"));
        }

        Ok(path)
    }

    /// `get_thumbnail_path( hash )`
    pub fn get_thumbnail_path(&self, hash: &[u8]) -> HydrusResult<PathBuf> {
        let path = self.get_expected_thumbnail_path(hash);

        if !path.exists() {
            return Err(HydrusException::new(
                ErrorKind::NotFound,
                "Thumbnail not found!",
            ));
        }

        Ok(path)
    }

    /// `iterate_all_paths( file_type )`, collected eagerly like the other
    /// generator ports in `hydrus_core`.
    ///
    /// Missing prefix directories are an error, matching `os.listdir` in the
    /// Python source. The server creates every prefix during database setup.
    pub fn iterate_all_paths(&self, file_type: ServerFileType) -> HydrusResult<Vec<PathBuf>> {
        let mut paths = Vec::new();

        for prefix in hydrus_files_physical_storage::iterate_prefixes("", 2) {
            let directory = self.files_dir.join(prefix);

            let entries = fs::read_dir(&directory).map_err(|error| {
                io_error(
                    ErrorKind::DirectoryMissing,
                    format!("Could not list the server storage directory \"{}\"", directory.display()),
                    error,
                )
            })?;

            for entry in entries {
                let entry = entry.map_err(|error| {
                    io_error(
                        ErrorKind::DirectoryMissing,
                        format!("Could not read an entry in \"{}\"", directory.display()),
                        error,
                    )
                })?;

                let path = entry.path();

                if file_type.includes(&path) {
                    paths.push(path);
                }
            }
        }

        paths.sort();

        Ok(paths)
    }
}
