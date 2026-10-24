# Hydrus Server Porting Plan

Current state: the server starts, creates its database and file storage, opens
the session manager, and can render a resource tree in-process. It does not yet
accept real HTTP/TLS requests.

## 1. HTTP/TLS listener and controller loop — mostly done

- [x] Add the async HTTP/TLS runtime and dependencies.
- [x] Load `server.crt`/`server.key` through `hydrus_core` TLS settings.
- [x] Bind the administration service and IPv4 listeners.
- [x] Convert HTTP requests into `HydrusRequest`.
- [x] Parse query strings, JSON/network bodies, headers, cookies and client IP.
- [x] Convert `RenderPlan` into real HTTP responses, including files, ranges,
      cookies and CORS headers.
- [x] Add the controller run loop, Ctrl+C handling and graceful shutdown.
- [ ] Add IPv6 dual-stack binding.
- [ ] Add request-size and `FatHTTPChannel` limits.
- [ ] Add an automated integration test over a real local TLS socket.
      A manual smoke test currently verifies `GET /busy` and `POST /shutdown`.

## 2. Controller lifecycle

- [ ] Integrate with the core `HydrusController`/`ControllerHooks`.
- [ ] Implement `init_model`, `init_view`, `run`, `exit` and shutdown.
- [ ] Start, stop and restart services.
- [ ] Implement `process_starting_action` and `shutdown_sibling_instance`.
- [ ] Implement daemons: `sync_repositories`, `save_dirty_objects`,
      `deferred_physical_deletes`, `nullify_history`.
- [ ] Implement `set_services`, `restart_services` and bandwidth checks.
- [ ] Implement backup, analyze and vacuum tasks.
- [ ] Add the running-file lifecycle.

## 3. ServerDB files and repository metadata

- [ ] Port `_add_file`, master hash IDs and `files_info`.
- [ ] Port physical file/thumbnail storage and deferred deletes.
- [ ] Port master tag IDs and tag lookup.
- [ ] Port IP address tracking.
- [ ] Port service-info creation, updates and regeneration.
- [ ] Port repository table creation migrations for existing databases.

## 4. Repository content

- [ ] Port repository add/delete files.
- [ ] Port mappings and mapping petitions.
- [ ] Port tag parents and tag siblings.
- [ ] Port file/mapping petition summaries and approval.
- [ ] Port `ClientToServerUpdate` processing.
- [ ] Port `Metadata` and update generation.
- [ ] Port `nullify_history`.
- [ ] Port account reward processing.

## 5. Account and option mutations

- [ ] Port account information and account-key-from-content.
- [ ] Port account-type changes.
- [ ] Port ban, unban, expiry, message and delete-all-content.
- [ ] Port options nullification/update period changes.
- [ ] Port account-type create/update/delete.
- [ ] Refresh account/session caches after mutations.

## 6. Remaining resources

- [ ] `options_nullification_period` and `options_update_period`.
- [ ] `other_account`, `account_info`, `account_key_from_content`.
- [ ] All `modify_account_*` resources.
- [ ] `maintenance_regen_service_info`, `backup`, `lock_on`, `lock_off`,
      `restart_services`, `services` POST, `vacuum`.
- [ ] `num_petitions`, `petition`, `petitions_summary`, `update`, `metadata`.
- [ ] `file`, `ip`, `thumbnail`, `tag_filter`.
- [ ] Replace the 501 fallback with real handlers.

## 7. Production hardening

- [ ] Add real database version migrations.
- [ ] Add concurrency and shutdown tests.
- [ ] Add large-file, range and upload tests.
- [ ] Add logging and request diagnostics parity.
- [ ] Add CLI argument parity and db-path selection.
- [ ] Add packaging/service-runner documentation.
