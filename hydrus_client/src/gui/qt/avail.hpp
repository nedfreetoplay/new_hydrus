#pragma once

// WS-15.3.2b: scalar getters implemented in Rust (api.rs shims). C++ passes
// widget state and applies the returned value, never an inline duplicate.

#include "rust/cxx.h"

#include <cstdint>

class HydrusMainWindow;
enum class UiAction : std::int32_t;

// Export phrase validation, static dir, empty-state strings (WS-15.3.2a).
rust::String hydrus_validate_export_phrase(rust::Str phrase) noexcept;
rust::String hydrus_static_dir() noexcept;
rust::String hydrus_no_local_tag_service_tooltip() noexcept;
rust::String hydrus_no_local_tag_service_label() noexcept;
UiAction action_from_str(rust::Str action) noexcept;

// Availability/visibility predicates (gui/app/availability.rs).
bool hydrus_avail_max_pending(rust::Str operation) noexcept;
bool hydrus_avail_better_side(rust::Str action) noexcept;
bool hydrus_avail_same_tab(rust::Str action) noexcept;
bool hydrus_avail_permission_list(bool permits_everything) noexcept;
int32_t hydrus_avail_comparator_panel(rust::Str kind) noexcept;
bool hydrus_avail_manager_edit(int64_t selected) noexcept;
bool hydrus_avail_manager_delete(int64_t selected) noexcept;
int64_t hydrus_avail_file_limit_value(bool unlimited, int64_t value) noexcept;
bool hydrus_avail_file_limit(bool unlimited) noexcept;
bool hydrus_avail_downloader_row(rust::Str kind, rust::Str row) noexcept;
bool hydrus_avail_downloader_start_enabled(bool running) noexcept;
bool hydrus_avail_downloader_highlight_subject(bool page_url_empty) noexcept;
bool hydrus_avail_second_search(rust::Str dupe_search_type) noexcept;
bool hydrus_avail_search_distance(rust::Str pixel_dupes) noexcept;
bool hydrus_avail_sort_ascending_visible(rust::Str sort_type) noexcept;
bool hydrus_avail_checked(bool checked) noexcept;
bool hydrus_avail_move_destination(rust::Str action) noexcept;
bool hydrus_avail_export_delete(rust::Str export_type) noexcept;
bool hydrus_avail_favourite_editable(rust::Str key) noexcept;
bool hydrus_avail_good_count(int64_t good_count) noexcept;
bool hydrus_avail_tagging_services(int64_t count) noexcept;

// Layout metrics (WS-15.3.2c, gui/app/layout_metrics.rs): C++ passes an
// index/state and applies the returned number, never a literal.
int64_t hydrus_metric_gallery_width(int64_t index) noexcept;
int64_t hydrus_metric_watcher_width(int64_t index) noexcept;
int64_t hydrus_metric_export_width(int64_t index) noexcept;
int64_t hydrus_metric_import_width(int64_t index) noexcept;
int64_t hydrus_metric_preview_width() noexcept;
int64_t hydrus_metric_review_width() noexcept;
int64_t hydrus_metric_scale_clamp(int64_t scale) noexcept;
int64_t hydrus_metric_file_limit_clamp(int64_t value) noexcept;
int64_t hydrus_metric_files_progress(int64_t done, int64_t total) noexcept;
int64_t hydrus_metric_autocomplete_rows(int64_t count) noexcept;

// Sorting/grouping/summary cells (WS-15.3.2d, gui/app/sort_group.rs): single
// source of truth for row order, seed timestamps and summary cells.
rust::Vec<int64_t> hydrus_sort_order_strings(rust::Vec<rust::String> keys, bool ascending) noexcept;
rust::Vec<int64_t> hydrus_sort_order_ints(rust::Vec<int64_t> keys, bool ascending) noexcept;
rust::Vec<int64_t> hydrus_sort_selected_rows(rust::Vec<int64_t> rows) noexcept;
rust::String hydrus_seed_time_text(int64_t seconds) noexcept;
rust::Vec<int64_t> hydrus_sort_group_rows(int64_t count, int64_t group_size) noexcept;
rust::String hydrus_comparator_number_text(double value) noexcept;
