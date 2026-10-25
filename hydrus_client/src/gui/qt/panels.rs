//! Renders [`MainViewModel`] panels into the Qt widgets from `bridge.hpp`.
//!
//! This module is deliberately dumb: it turns view-model values into widgets
//! and never builds application state.

use std::pin::Pin;

use cxx_qt_lib as _;

use super::bridge::ffi;
use crate::gui::view_model::{
    CanvasViewModel, FormViewModel, ListViewModel, MediaViewModel, PanelViewModel, WidgetViewModel,
};

type Window<'a> = Pin<&'a mut ffi::HydrusMainWindow>;

pub(crate) fn add_panels(mut window: Window<'_>, panels: &[PanelViewModel]) {
    for panel in panels {
        match panel {
            PanelViewModel::Form(form) => add_form(window.as_mut(), form),
            PanelViewModel::ScrolledForm(form) => add_scrolled_form(window.as_mut(), form),
            PanelViewModel::List(list) => add_list(window.as_mut(), list),
            PanelViewModel::Media(media) => add_media(window.as_mut(), media),
            PanelViewModel::Canvas(canvas) => add_canvas(window.as_mut(), canvas),
        }
    }
}

fn add_form(mut window: Window<'_>, form: &FormViewModel) {
    let mut panel = ffi::new_hydrus_panel();

    for widget in &form.widgets {
        match widget {
            WidgetViewModel::Label(text) => ffi::hydrus_panel_add_label(panel.pin_mut(), text),
            WidgetViewModel::LineEdit(text) => {
                ffi::hydrus_panel_add_line_edit(panel.pin_mut(), text)
            }
            WidgetViewModel::Button(text) => ffi::hydrus_panel_add_button(panel.pin_mut(), text),
            WidgetViewModel::Checkbox { text, checked } => {
                ffi::hydrus_panel_add_checkbox(panel.pin_mut(), text, *checked);
            }
            WidgetViewModel::Stretch => ffi::hydrus_panel_add_stretch(panel.pin_mut()),
        }
    }

    ffi::hydrus_add_page_widget(window.as_mut(), &form.title, panel.pin_mut());
}

fn add_scrolled_form(mut window: Window<'_>, form: &FormViewModel) {
    let mut panel = ffi::new_hydrus_scrolled_panel();

    for widget in &form.widgets {
        match widget {
            WidgetViewModel::Label(text) => ffi::hydrus_scrolled_add_label(panel.pin_mut(), text),
            WidgetViewModel::LineEdit(text) => {
                ffi::hydrus_scrolled_add_line_edit(panel.pin_mut(), text);
            }
            WidgetViewModel::Button(text) => ffi::hydrus_scrolled_add_button(panel.pin_mut(), text),
            WidgetViewModel::Checkbox { text, checked } => {
                ffi::hydrus_scrolled_add_checkbox(panel.pin_mut(), text, *checked);
            }
            WidgetViewModel::Stretch => ffi::hydrus_scrolled_add_stretch(panel.pin_mut()),
        }
    }

    ffi::hydrus_add_scrolled_page(window.as_mut(), &form.title, panel.pin_mut());
}

fn add_list(mut window: Window<'_>, list: &ListViewModel) {
    let mut panel = ffi::new_hydrus_list_panel();
    ffi::hydrus_list_set_columns(panel.pin_mut(), &list.columns);

    for row in &list.rows {
        ffi::hydrus_list_add_row(panel.pin_mut(), row);
    }

    ffi::hydrus_add_list_page(window.as_mut(), &list.title, panel.pin_mut());
}

fn add_media(mut window: Window<'_>, media: &MediaViewModel) {
    let mut panel = ffi::new_hydrus_media_panel();

    for item in &media.items {
        ffi::hydrus_media_panel_add_item(panel.pin_mut(), item);
    }

    ffi::hydrus_add_media_page(window.as_mut(), &media.title, panel.pin_mut());
}

fn add_canvas(mut window: Window<'_>, canvas: &CanvasViewModel) {
    let mut canvas_widget = ffi::new_hydrus_canvas();
    ffi::hydrus_add_canvas_page(window.as_mut(), &canvas.title, canvas_widget.pin_mut());
}
