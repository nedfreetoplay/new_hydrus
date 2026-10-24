impl ServerDB {


    /// `_get_reason_id( reason )`
    pub fn get_reason_id(&self, reason: &str) -> HydrusResult<i64> {
        let db = self.db.db_base()?;

        if let Some(reason_id) = db.query_one(
            "SELECT reason_id FROM reasons WHERE reason = ?;",
            &[&reason],
            |row| row.get::<_, i64>(0),
        )? {
            return Ok(reason_id);
        }

        db.execute(
            "INSERT INTO reasons ( reason ) VALUES ( ? );",
            &[&reason],
        )?;

        Ok(db.get_last_row_id())
    }

    /// `_add_file( file_dict )`
    pub fn add_file(&self, file: &ServerFileInput) -> HydrusResult<i64> {
        let master_hash_id = self.get_master_hash_id(&file.hash)?;
        let db = self.db.db_base()?;

        if db
            .query_one(
                "SELECT 1 FROM files_info WHERE master_hash_id = ?;",
                &[&master_hash_id],
                |row| row.get::<_, i64>(0),
            )?
            .is_none()
        {
            db.execute(
                "INSERT OR IGNORE INTO files_info ( master_hash_id, size, mime, width, height, duration, num_frames, num_words ) VALUES ( ?, ?, ?, ?, ?, ?, ?, ? );",
                &[
                    &master_hash_id,
                    &(file.size as i64),
                    &i64::from(file.mime.as_int()),
                    &file.width.map(i64::from),
                    &file.height.map(i64::from),
                    &file.duration_ms,
                    &file.num_frames.map(i64::from),
                    &file.num_words,
                ],
            )?;
        }

        let dest_path = self.files.get_expected_file_path(&file.hash);

        if !dest_path.exists() {
            hydrus_core::hydrus_paths::mirror_file(&file.path, &dest_path)?;
        }

        if let Some(thumbnail) = &file.thumbnail {
            let thumbnail_path = self.files.get_expected_thumbnail_path(&file.hash);

            if !thumbnail_path.exists() {
                if let Some(parent) = thumbnail_path.parent() {
                    hydrus_core::hydrus_paths::make_sure_directory_exists(parent)?;
                }

                std::fs::write(&thumbnail_path, thumbnail).map_err(|error| {
                    HydrusException::new(
                        ErrorKind::FileMissing,
                        format!(
                            "Could not write the thumbnail \"{}\": {error}",
                            thumbnail_path.display()
                        ),
                    )
                })?;
            }
        }

        Ok(master_hash_id)
    }

    /// `_get_deferred_physical_delete()`
    pub fn get_deferred_physical_delete(
        &self,
    ) -> HydrusResult<(Option<Vec<u8>>, Option<Vec<u8>>)> {
        let db = self.db.db_base()?;

        let file_hash = db
            .query_one(
                "SELECT master_hash_id FROM deferred_physical_file_deletes LIMIT 1;",
                &[],
                |row| row.get::<_, i64>(0),
            )?
            .map(|master_hash_id| self.get_hash(master_hash_id))
            .transpose()?;

        let thumbnail_hash = db
            .query_one(
                "SELECT master_hash_id FROM deferred_physical_thumbnail_deletes LIMIT 1;",
                &[],
                |row| row.get::<_, i64>(0),
            )?
            .map(|master_hash_id| self.get_hash(master_hash_id))
            .transpose()?;

        Ok((file_hash, thumbnail_hash))
    }

    /// `_clear_deferred_physical_delete( file_hash, thumbnail_hash )`
    pub fn clear_deferred_physical_delete(
        &self,
        file_hash: Option<&[u8]>,
        thumbnail_hash: Option<&[u8]>,
    ) -> HydrusResult<()> {
        let db = self.db.db_base()?;

        if let Some(file_hash) = file_hash {
            let master_hash_id = self.get_master_hash_id(file_hash)?;

            db.execute(
                "DELETE FROM deferred_physical_file_deletes WHERE master_hash_id = ?;",
                &[&master_hash_id],
            )?;
        }

        if let Some(thumbnail_hash) = thumbnail_hash {
            let master_hash_id = self.get_master_hash_id(thumbnail_hash)?;

            db.execute(
                "DELETE FROM deferred_physical_thumbnail_deletes WHERE master_hash_id = ?;",
                &[&master_hash_id],
            )?;
        }

        Ok(())
    }

    /// `_defer_files_delete_if_now_orphan( master_hash_ids, definitely_no_thumbnails, ignore_service_id )`
    pub fn defer_files_delete_if_now_orphan(
        &self,
        master_hash_ids: &[i64],
        definitely_no_thumbnails: bool,
        ignore_service_id: Option<i64>,
    ) -> HydrusResult<usize> {
        let orphan_master_hash_ids =
            self.filter_orphan_master_hash_ids(master_hash_ids, ignore_service_id)?;

        if orphan_master_hash_ids.is_empty() {
            return Ok(0);
        }

        let db = self.db.db_base()?;

        for master_hash_id in &orphan_master_hash_ids {
            db.execute(
                "INSERT OR IGNORE INTO deferred_physical_file_deletes ( master_hash_id ) VALUES ( ? );",
                &[master_hash_id],
            )?;

            if !definitely_no_thumbnails {
                db.execute(
                    "INSERT OR IGNORE INTO deferred_physical_thumbnail_deletes ( master_hash_id ) VALUES ( ? );",
                    &[master_hash_id],
                )?;
            }
        }

        Ok(orphan_master_hash_ids.len())
    }
}
