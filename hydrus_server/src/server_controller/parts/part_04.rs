use hydrus_core::networking::hydrus_network::{
    ServerBusyGuard, ServerServiceController,
};

struct ControllerBusyGuard {
    busy: Arc<AtomicBool>,
}

impl ServerBusyGuard for ControllerBusyGuard {}

impl Drop for ControllerBusyGuard {
    fn drop(&mut self) {
        self.busy.store(false, Ordering::Relaxed);
    }
}

impl ServerServiceController for Controller {
    fn try_lock_server_busy(&self) -> Option<Box<dyn ServerBusyGuard>> {
        self.busy
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .ok()
            .map(|_| {
                Box::new(ControllerBusyGuard {
                    busy: Arc::clone(&self.busy),
                }) as Box<dyn ServerBusyGuard>
            })
    }

    fn nullify_history(
        &self,
        service_key: &[u8],
        begin: i64,
        end: i64,
    ) -> HydrusResult<()> {
        self.database()?
            .repository_nullify_history(service_key, begin, end)
    }

    fn create_update(
        &self,
        service_key: &[u8],
        begin: i64,
        end: i64,
    ) -> HydrusResult<Vec<Vec<u8>>> {
        self.database()?
            .repository_create_update(service_key, begin, end)
    }

    fn publish(&self, _topic: &str) {
        // Daemon notifications are polled by the controller loop.
    }
}

#[allow(dead_code)]
impl Controller {
    fn database(&self) -> HydrusResult<Arc<ServerDB>> {
        self.db.get().cloned().ok_or_else(|| {
            HydrusException::new(ErrorKind::Unknown, "The server database is not open!")
        })
    }

    /// `sync_repositories` daemon.
    pub fn sync_repositories(self: &Arc<Self>) -> HydrusResult<bool> {
        let db = self.open_database()?;
        let mut created = false;

        for service in db.get_services(Some(&[
            ServiceType::FileRepository,
            ServiceType::TagRepository,
        ]))? {
            let metadata = service.get_metadata()?;

            while metadata.update_due(false) {
                let begin = metadata.get_next_update_begin();
                let update_period = service.get_update_period();
                let end = begin + update_period;
                let update_hashes = db.repository_create_update(
                    service.get_service_key(),
                    begin,
                    end,
                )?;

                metadata.append_update(
                    update_hashes,
                    begin,
                    end,
                    end + update_period,
                );

                db.save_service(&service)?;

                created = true;

                if self.shutdown_requested.load(Ordering::Relaxed) {
                    break;
                }
            }
        }

        Ok(created)
    }

    /// `save_dirty_objects` daemon.
    pub fn save_dirty_objects(self: &Arc<Self>) -> HydrusResult<()> {
        let db = self.open_database()?;

        for service in db.get_services(None)? {
            if service.is_dirty() {
                db.save_service(&service)?;
            }
        }

        Ok(())
    }

    /// `deferred_physical_deletes` daemon.
    pub fn process_deferred_physical_deletes(self: &Arc<Self>) -> HydrusResult<()> {
        let db = self.open_database()?;
        let files = ServerFiles::new(db.get_files_dir());

        loop {
            let (file_hash, thumbnail_hash) = db.get_deferred_physical_delete()?;

            if file_hash.is_none() && thumbnail_hash.is_none() {
                break;
            }

            if let Some(file_hash) = &file_hash {
                let path = files.get_expected_file_path(file_hash);

                if path.exists() {
                    hydrus_core::hydrus_paths::delete_path(&path);
                }
            }

            if let Some(thumbnail_hash) = &thumbnail_hash {
                let path = files.get_expected_thumbnail_path(thumbnail_hash);

                if path.exists() {
                    hydrus_core::hydrus_paths::delete_path(&path);
                }
            }

            db.clear_deferred_physical_delete(
                file_hash.as_deref(),
                thumbnail_hash.as_deref(),
            )?;
        }

        Ok(())
    }

    /// `nullify_history` daemon.
    pub fn nullify_history(self: &Arc<Self>) -> HydrusResult<()> {
        let db = self.open_database()?;

        for service in db.get_services(Some(&[
            ServiceType::FileRepository,
            ServiceType::TagRepository,
        ]))? {
            service.nullify_history()?;

            if service.is_dirty() {
                db.save_service(&service)?;
            }
        }

        Ok(())
    }

    /// One pass of every background maintenance task.
    pub fn run_daemons(self: &Arc<Self>) -> HydrusResult<()> {
        self.sync_repositories()?;
        self.process_deferred_physical_deletes()?;
        self.nullify_history()?;
        self.save_dirty_objects()?;

        Ok(())
    }
}
