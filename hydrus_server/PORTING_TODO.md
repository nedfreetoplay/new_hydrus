# Hydrus Server Porting Plan

Current state: the server starts, creates its database and file storage, opens
the session manager, and can render a resource tree in-process. It does not yet
accept real HTTP/TLS requests.

## 1. HTTP/TLS listener and controller loop — done

- [x] Add the async HTTP/TLS runtime and dependencies.
- [x] Load `server.crt`/`server.key` through `hydrus_core` TLS settings.
- [x] Bind the administration service and IPv4 listeners.
- [x] Convert HTTP requests into `HydrusRequest`.
- [x] Parse query strings, JSON/network bodies, headers, cookies and client IP.
- [x] Convert `RenderPlan` into real HTTP responses, including files, ranges,
      cookies and CORS headers.
- [x] Add the controller run loop, Ctrl+C handling and graceful shutdown.
- [x] Add IPv6 dual-stack binding with v6-only sockets.
- [x] Add request-size and `FatHTTPChannel` limits.
- [x] Add an automated integration test over a real local TLS socket.

## 2. Controller lifecycle

- [ ] Integrate with the core `HydrusController`/`ControllerHooks`.
- [x] Implement `init_model`, `init_view`, `run`, `exit` and shutdown.
- [x] Start and stop listeners through the async controller loop.
- [ ] Implement `process_starting_action` and `shutdown_sibling_instance`.
- [ ] Implement daemons: `sync_repositories`, `save_dirty_objects`,
      `deferred_physical_deletes`, `nullify_history`.
- [x] Implement `set_services`, `restart_services` and bandwidth checks (basic).
- [x] Implement backup, analyze and vacuum tasks (backup is complete; live
      vacuum commits and defers file compaction to shutdown).
- [x] Add the running-file lifecycle.

## 3. ServerDB files and repository metadata

- [x] Port `_add_file`, master hash IDs and `files_info`.
- [x] Port physical file/thumbnail storage.
- [x] Port deferred physical deletes.
- [x] Port master tag IDs and tag lookup.
- [x] Port IP address tracking.
- [x] Port service-info creation, updates and regeneration.
      File and tag repository counts are regenerated from their repository
      tables, and incremental `update_service_info` deltas are supported.
- [ ] Port repository table creation migrations for existing databases.

## 4. Repository content

- [x] Port repository add/delete files.
- [x] Port `repository_has_file` and the repository service-hash map.
- [x] Port repository mappings and current/deleted mapping tables.
- [x] Port mapping petitions.
- [x] Port current/deleted tag parents and tag siblings.
- [x] Port pending and petitioned tag parent/sibling flows, including deny operations.
- [x] Port file and mapping petition creation/denial.
- [x] Port file, mapping, tag-parent and tag-sibling petition summaries.
- [x] Port petition approval/denial processing.
- [x] Port basic `ClientToServerUpdate` processing for files, mappings and tag relations.
- [ ] Port `Metadata` and update generation.
- [ ] Port `nullify_history`.
- [ ] Port account reward processing.

## 5. Account and option mutations

- [x] Port account information and account-key-from-content.
- [x] Port account-type changes.
- [x] Port ban, unban, expiry, message and delete-all-content.
- [x] Port options nullification/update period changes.
- [x] Port account-type create/update/delete.
- [ ] Refresh account/session caches after mutations.

## 6. Remaining resources

- [x] `options_nullification_period` and `options_update_period`.
- [x] `other_account`, `account_info`, `account_key_from_content`.
- [x] All `modify_account_*` resources.
- [x] `maintenance_regen_service_info`.
- [x] `backup`, `lock_on`, `lock_off`, `restart_services`, `vacuum`.
- [ ] `services` POST.
- [x] `num_petitions`, `petitions_summary`.
- [ ] `petition`, `update`, `metadata`.
- [x] `file` GET, `ip`, `thumbnail`, `tag_filter` GET.
- [ ] `file` POST and `tag_filter` POST.
- [ ] Replace the remaining 501 fallback with real handlers.

## 7. Production hardening

- [ ] Add real database version migrations.
- [ ] Add concurrency and shutdown tests.
- [ ] Add large-file, range and upload tests.
- [ ] Add logging and request diagnostics parity.
- [ ] Add CLI argument parity and db-path selection.
- [ ] Add packaging/service-runner documentation.
