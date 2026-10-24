//! Split out from the parent module.

use crate::client_globals::ClientControllerHandle;
use crate::gui::pages::{Page, PageType, PagesNotebook};
use crate::gui::session::GuiSession;

use super::*;

pub(super) fn default_session() -> GuiSession {
    let mut notebook = PagesNotebook::new();
    notebook.add_page(Page::new("Welcome", PageType::Search));

    GuiSession::from_notebook(&notebook)
}

impl ClientControllerHandle for Controller {
    fn publish_message(&self, message: &str) {
        crate::gui::pubsub_bridge::post("message", message);

        if let Some(logger) = crate::client_boot::logger() {
            logger.write(message);
            logger.write("\n");
        } else {
            println!("{message}");
        }
    }
}
