//! Canvas navigation state and media modal actions.
//!
//! Mirrors the interaction helpers in `ClientGUICanvas.py` and
//! `ClientGUIMediaModalActions.py`; the actual drawing stays in Qt.

use hydrus_core::hydrus_constants::ContentType;

use crate::metadata::{ContentAction, ContentUpdate, ContentUpdatePackage};

#[derive(Debug, Clone, PartialEq)]
pub struct CanvasState {
    pub zoom: f64,
    pub rotation_degrees: i32,
    pub pan: (f64, f64),
    pub fitted: bool,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            rotation_degrees: 0,
            pan: (0.0, 0.0),
            fitted: false,
        }
    }
}

impl CanvasState {
    pub const MIN_ZOOM: f64 = 0.05;
    pub const MAX_ZOOM: f64 = 20.0;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn zoom_in(&mut self) {
        self.set_zoom(self.zoom * 1.25);
    }

    pub fn zoom_out(&mut self) {
        self.set_zoom(self.zoom / 1.25);
    }

    pub fn set_zoom(&mut self, zoom: f64) {
        self.zoom = zoom.clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        self.fitted = false;
    }

    pub fn fit(&mut self) {
        self.fitted = true;
        self.zoom = 1.0;
    }

    pub fn rotate_left(&mut self) {
        self.rotation_degrees = (self.rotation_degrees - 90).rem_euclid(360);
    }

    pub fn rotate_right(&mut self) {
        self.rotation_degrees = (self.rotation_degrees + 90).rem_euclid(360);
    }

    pub fn pan_by(&mut self, dx: f64, dy: f64) {
        self.pan.0 += dx;
        self.pan.1 += dy;
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaModalAction {
    Archive,
    Inbox,
    Delete,
    Undelete,
}

impl MediaModalAction {
    pub fn content_action(self) -> ContentAction {
        match self {
            Self::Archive | Self::Inbox | Self::Undelete => ContentAction::Add,
            Self::Delete => ContentAction::Delete,
        }
    }
}

/// Builds the content update for archive/inbox/delete/undelete actions.
pub fn media_modal_update(
    action: MediaModalAction,
    service_key: impl Into<Vec<u8>>,
    hashes: impl IntoIterator<Item = Vec<u8>>,
) -> ContentUpdatePackage {
    ContentUpdatePackage::from_updates(vec![
        ContentUpdate::new(
            action.content_action(),
            service_key.into(),
            ContentType::Files,
        )
        .with_hashes(hashes),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_is_clamped_and_fit_resets_it() {
        let mut canvas = CanvasState::new();

        for _ in 0..100 {
            canvas.zoom_in();
        }
        assert_eq!(canvas.zoom, CanvasState::MAX_ZOOM);

        for _ in 0..200 {
            canvas.zoom_out();
        }
        assert_eq!(canvas.zoom, CanvasState::MIN_ZOOM);

        canvas.fit();
        assert!(canvas.fitted);
        assert_eq!(canvas.zoom, 1.0);
    }

    #[test]
    fn rotation_wraps_both_ways() {
        let mut canvas = CanvasState::new();

        canvas.rotate_left();
        assert_eq!(canvas.rotation_degrees, 270);

        canvas.rotate_right();
        canvas.rotate_right();
        canvas.rotate_right();
        canvas.rotate_right();
        assert_eq!(canvas.rotation_degrees, 270);
    }

    #[test]
    fn modal_actions_build_content_updates() {
        let package = media_modal_update(
            MediaModalAction::Archive,
            b"my files".to_vec(),
            [vec![1, 2]],
        );

        assert_eq!(package.updates.len(), 1);
        assert_eq!(package.updates[0].action, ContentAction::Add);
        assert_eq!(package.updates[0].content_type, ContentType::Files);
        assert_eq!(package.updates[0].hashes, vec![vec![1, 2]]);

        let delete = media_modal_update(MediaModalAction::Delete, b"trash".to_vec(), [vec![3]]);
        assert_eq!(delete.updates[0].action, ContentAction::Delete);
    }
}
