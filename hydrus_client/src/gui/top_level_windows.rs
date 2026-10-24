//! Top-level window state from `hydrus/client/gui/ClientGUITopLevelWindows.py`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    MainFrame,
    Frame,
    Dialog,
    NullipotentDialog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowState {
    Hidden,
    Shown,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Default for WindowGeometry {
    fn default() -> Self {
        Self {
            x: -1,
            y: -1,
            width: 800,
            height: 600,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopLevelWindow {
    pub title: String,
    pub window_type: WindowType,
    pub geometry: WindowGeometry,
    pub state: WindowState,
    pub modal: bool,
    pub result: Option<i32>,
}

impl TopLevelWindow {
    pub fn new(title: impl Into<String>, window_type: WindowType) -> Self {
        Self {
            title: title.into(),
            window_type,
            geometry: WindowGeometry::default(),
            state: WindowState::Hidden,
            modal: window_type == WindowType::Dialog,
            result: None,
        }
    }

    pub fn show(&mut self) {
        self.state = WindowState::Shown;
    }

    pub fn hide(&mut self) {
        if self.state != WindowState::Closed {
            self.state = WindowState::Hidden;
        }
    }

    pub fn close(&mut self) {
        self.state = WindowState::Closed;
    }

    pub fn is_shown(&self) -> bool {
        self.state == WindowState::Shown
    }

    pub fn is_closed(&self) -> bool {
        self.state == WindowState::Closed
    }

    pub fn set_geometry(&mut self, geometry: WindowGeometry) {
        self.geometry = geometry;
    }

    pub fn accept(&mut self) {
        self.result = Some(1);
        self.close();
    }

    pub fn reject(&mut self) {
        self.result = Some(0);
        self.close();
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_moves_between_states() {
        let mut window = TopLevelWindow::new("test", WindowType::Frame);

        assert!(!window.is_shown());

        window.show();
        assert!(window.is_shown());

        window.hide();
        assert!(!window.is_shown());
        assert!(!window.is_closed());

        window.close();
        assert!(window.is_closed());
    }

    #[test]
    fn dialogs_are_modal_by_default() {
        assert!(TopLevelWindow::new("d", WindowType::Dialog).modal);
        assert!(!TopLevelWindow::new("f", WindowType::Frame).modal);
    }

    #[test]
    fn accept_and_reject_set_results() {
        let mut window = TopLevelWindow::new("d", WindowType::Dialog);

        window.accept();
        assert_eq!(window.result, Some(1));
        assert!(window.is_closed());

        let mut window = TopLevelWindow::new("d", WindowType::Dialog);
        window.reject();
        assert_eq!(window.result, Some(0));
    }

    #[test]
    fn geometry_can_be_restored() {
        let mut window = TopLevelWindow::new("d", WindowType::Dialog);
        let geometry = WindowGeometry {
            x: 10,
            y: 20,
            width: 640,
            height: 480,
        };

        window.set_geometry(geometry.clone());

        assert_eq!(window.geometry, geometry);
    }
}
