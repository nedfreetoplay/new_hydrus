//! Qt-independent helpers from `hydrus/client/gui/ClientGUIFunctions.py`.

pub const NUM_CHARS_FOR_WIDTH_CALCULATIONS: i32 = 32;
pub const MAGIC_TEXT_PADDING: f64 = 1.1;

pub type Colour = (u8, u8, u8);

pub fn colour_is_bright(colour: Colour) -> bool {
    let luminance = 0.299 * colour.0 as f64 + 0.587 * colour.1 as f64 + 0.114 * colour.2 as f64;

    luminance > 127.5
}

pub fn colour_is_greyish(colour: Colour) -> bool {
    let maximum = colour.0.max(colour.1).max(colour.2);
    let minimum = colour.0.min(colour.1).min(colour.2);

    (maximum - minimum) < 10
}

pub fn get_lighter_darker_colour(colour: Colour, intensity: i32) -> Colour {
    if colour_is_bright(colour) {
        darken(colour, intensity)
    } else {
        lighten(colour, intensity)
    }
}

pub fn get_different_lighter_darker_colour(colour: Colour, intensity: i32) -> Colour {
    if colour_is_greyish(colour) {
        let delta = (intensity * 3).clamp(-255, 255) as i16;

        (
            (colour.0 as i16 + delta).clamp(0, 255) as u8,
            (colour.1 as i16 + delta).clamp(0, 255) as u8,
            (colour.2 as i16 + delta).clamp(0, 255) as u8,
        )
    } else {
        get_lighter_darker_colour(colour, intensity)
    }
}

fn lighten(colour: Colour, intensity: i32) -> Colour {
    let factor = 1.0 + (intensity.max(0) as f64 / 100.0);

    (
        (colour.0 as f64 * factor).min(255.0) as u8,
        (colour.1 as f64 * factor).min(255.0) as u8,
        (colour.2 as f64 * factor).min(255.0) as u8,
    )
}

fn darken(colour: Colour, intensity: i32) -> Colour {
    let factor = 1.0 - (intensity.max(0) as f64 / 100.0).min(1.0);

    (
        (colour.0 as f64 * factor) as u8,
        (colour.1 as f64 * factor) as u8,
        (colour.2 as f64 * factor) as u8,
    )
}

pub fn convert_text_to_pixel_width(char_width: f64, char_cols: f64) -> i32 {
    (char_width * char_cols * MAGIC_TEXT_PADDING).round() as i32
}

pub fn convert_pixels_to_text_width(char_width: f64, pixels: i32, round_down: bool) -> i32 {
    if char_width <= 0.0 {
        return 0;
    }

    let value = pixels as f64 / (char_width * MAGIC_TEXT_PADDING);

    if round_down {
        value.floor() as i32
    } else {
        value.round() as i32
    }
}

pub fn convert_text_to_pixel_height(char_height: f64, char_rows: f64) -> i32 {
    (char_height * char_rows).round() as i32
}

pub fn escape_mnemonics(text: &str) -> String {
    text.replace('&', "&&")
}

pub fn wrap_tool_tip(text: &str, max_line_length: usize) -> String {
    if max_line_length == 0 {
        return text.to_string();
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= max_line_length {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines.join("\n")
}

/// `ClientGUIFunctionsImage` helpers: all conversions go through `image` so the
/// GUI layer never needs to know about Pillow/numpy.
pub fn scale_pixmap(
    image: &image::DynamicImage,
    width: u32,
    height: u32,
    keep_aspect: bool,
) -> image::DynamicImage {
    if keep_aspect {
        let (image_width, image_height) = (image.width(), image.height());
        let scale = (width as f64 / image_width as f64).min(height as f64 / image_height as f64);
        let target_width = (image_width as f64 * scale).round().max(1.0) as u32;
        let target_height = (image_height as f64 * scale).round().max(1.0) as u32;

        image.resize(
            target_width,
            target_height,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        image.resize_exact(width, height, image::imageops::FilterType::Lanczos3)
    }
}

pub fn image_to_rgba_bytes(image: &image::DynamicImage) -> Vec<u8> {
    image.to_rgba8().into_raw()
}

pub fn image_from_rgba_bytes(width: u32, height: u32, bytes: &[u8]) -> Option<image::DynamicImage> {
    image::RgbaImage::from_raw(width, height, bytes.to_vec()).map(image::DynamicImage::ImageRgba8)
}

#[cfg(test)]
mod tests;
