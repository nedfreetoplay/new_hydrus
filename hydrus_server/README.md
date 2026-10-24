# hydrus_server

The runnable Rust port of `hydrus_python/hydrus/server`.

## Build

```text
cargo build -p hydrus_server --release
```

The binary is `target/release/hydrus_server` (`hydrus_server.exe` on Windows).

## Run

```text
hydrus_server [DB_DIR] [--start|--stop|--restart|--exit] [--db-dir PATH]
```

- `DB_DIR` / `--db-dir PATH` selects the server database directory.
- `HYDRUS_SERVER_DB_DIR` is used when no path is given.
- The default is `./server_db`.
- `--stop` and `--restart` ask a running sibling instance to shut down using
  the ports recorded in its running file.
- `--exit` performs the starting-action check and exits without opening the
  database or listeners.

The server creates `server.db`, `server.master.db`, `server.mappings.db`,
`server.crt`/`server.key` (when TLS paths are configured) and `server_files/`
inside the database directory.

## Services

On first boot the database contains an administration service. Repository
services are added through the administration `/services` endpoint (the server
UI/client does this) and then listen on their configured ports.

Every service serves HTTPS. Requests use the regular Hydrus headers
(`Hydrus-Key`, `Hydrus-Session-Key`) and the network argument encoding.

## Background tasks

The controller loop runs a 60-second daemon pass:

- `sync_repositories` creates pending repository updates.
- `deferred_physical_deletes` removes orphaned files and thumbnails.
- `nullify_history` anonymises old repository history.
- `save_dirty_objects` persists dirty service dictionaries.

## Tests

```text
cargo test -p hydrus_server
cargo test -p hydrus_core --lib -- --test-threads=1
```

The server tests cover schema creation, repository content, update generation,
resource routing and a real local TLS listener.
