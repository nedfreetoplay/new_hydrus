impl ServerFiles {
    /// `get_all_hashes( file_type )`
    ///
    /// Python's thumbnail branch passes the `.thumbnail` suffix to
    /// `bytes.fromhex`; this port strips the suffix first, which is what the
    /// operation is actually meant to return.
    pub fn get_all_hashes(
        &self,
        file_type: ServerFileType,
    ) -> HydrusResult<HashSet<Vec<u8>>> {
        let mut hashes = HashSet::new();

        for path in self.iterate_all_paths(file_type)? {
            let name = path.file_name().and_then(OsStr::to_str).ok_or_else(|| {
                HydrusException::new(
                    ErrorKind::DamagedOrUnusualFile,
                    format!("The server storage path \"{}\" has no usable filename.", path.display()),
                )
            })?;

            let hash_text = match file_type {
                ServerFileType::File => name,
                ServerFileType::Thumbnail => name
                    .strip_suffix(".thumbnail")
                    .unwrap_or(name),
            };

            let hash = hex_decode(hash_text).map_err(|()| {
                HydrusException::new(
                    ErrorKind::DamagedOrUnusualFile,
                    format!("The server storage filename \"{name}\" is not a valid hash."),
                )
            })?;

            hashes.insert(hash);
        }

        Ok(hashes)
    }
}
