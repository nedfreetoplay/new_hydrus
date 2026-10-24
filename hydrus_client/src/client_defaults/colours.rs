//! Colour-set defaults from `ClientOptions._InitialiseDefaults`.

use serde_json::{Map, Value, json};

use crate::client_constants::*;

pub fn default_colours() -> Map<String, Value> {
    build(&[
        (COLOUR_THUMB_BACKGROUND, (255, 255, 255)),
        (COLOUR_THUMB_BACKGROUND_SELECTED, (217, 242, 255)),
        (COLOUR_THUMB_BACKGROUND_REMOTE, (32, 32, 36)),
        (COLOUR_THUMB_BACKGROUND_REMOTE_SELECTED, (64, 64, 72)),
        (COLOUR_THUMB_BORDER, (223, 227, 230)),
        (COLOUR_THUMB_BORDER_SELECTED, (1, 17, 26)),
        (COLOUR_THUMB_BORDER_REMOTE, (248, 208, 204)),
        (COLOUR_THUMB_BORDER_REMOTE_SELECTED, (227, 66, 52)),
        (COLOUR_THUMBGRID_BACKGROUND, (255, 255, 255)),
        (COLOUR_AUTOCOMPLETE_BACKGROUND, (235, 248, 255)),
        (COLOUR_MEDIA_BACKGROUND, (255, 255, 255)),
        (COLOUR_MEDIA_TEXT, (0, 0, 0)),
        (COLOUR_TAGS_BOX, (255, 255, 255)),
    ])
}

pub fn darkmode_colours() -> Map<String, Value> {
    build(&[
        (COLOUR_THUMB_BACKGROUND, (64, 64, 72)),
        (COLOUR_THUMB_BACKGROUND_SELECTED, (112, 128, 144)),
        (COLOUR_THUMB_BACKGROUND_REMOTE, (64, 13, 2)),
        (COLOUR_THUMB_BACKGROUND_REMOTE_SELECTED, (171, 39, 79)),
        (COLOUR_THUMB_BORDER, (145, 163, 176)),
        (COLOUR_THUMB_BORDER_SELECTED, (223, 227, 230)),
        (COLOUR_THUMB_BORDER_REMOTE, (248, 208, 204)),
        (COLOUR_THUMB_BORDER_REMOTE_SELECTED, (227, 66, 52)),
        (COLOUR_THUMBGRID_BACKGROUND, (52, 52, 52)),
        (COLOUR_AUTOCOMPLETE_BACKGROUND, (83, 98, 103)),
        (COLOUR_MEDIA_BACKGROUND, (52, 52, 52)),
        (COLOUR_MEDIA_TEXT, (112, 128, 144)),
        (COLOUR_TAGS_BOX, (35, 38, 41)),
    ])
}

fn build(entries: &[(i32, (u8, u8, u8))]) -> Map<String, Value> {
    entries
        .iter()
        .map(|(id, (red, green, blue))| {
            (id.to_string(), json!([red, green, blue]))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coloursets_have_every_python_colour() {
        assert_eq!(default_colours().len(), 13);
        assert_eq!(darkmode_colours().len(), 13);
    }

    #[test]
    fn known_colours_match_python() {
        let colours = default_colours();

        assert_eq!(
            colours[&COLOUR_THUMB_BACKGROUND.to_string()],
            json!([255, 255, 255])
        );
        assert_eq!(
            darkmode_colours()[&COLOUR_TAGS_BOX.to_string()],
            json!([35, 38, 41])
        );
    }
}
