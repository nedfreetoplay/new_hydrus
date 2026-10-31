# Hydrus Network static assets (vendored)

These files are vendored verbatim from the Hydrus Network Python client
`static/` directory for GUI parity (icons, QSS stylesheets, star shapes, mpv
configuration, build files, test data).

- Upstream: https://github.com/hydrusnetwork/hydrus (this checkout is a
  modified Hydrus tree used as the parity reference)
- Upstream software version: 692
- Upstream commit: 54acb5bdcd8a0b7ddd2ea040ccf326646ed3968f
- Files: 315, copied unchanged (names and structure preserved).

Do not edit the binary assets. The Rust client resolves this directory at
runtime via `hydrus_client::gui::app::static_assets::resolve_static_dir`
(env `HYDRUS_STATIC_DIR`, `<exe dir>/static`, then `<repo root>/static`).