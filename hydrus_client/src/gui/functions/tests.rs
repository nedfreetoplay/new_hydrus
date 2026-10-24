
use super::*;

#[test]
fn brightness_and_greyish_match_expectations() {
    assert!(colour_is_bright((255, 255, 255)));
    assert!(!colour_is_bright((0, 0, 0)));
    assert!(colour_is_greyish((100, 101, 99)));
    assert!(!colour_is_greyish((200, 20, 20)));
}

#[test]
fn light_and_dark_adjust_in_the_right_direction() {
    let lighter = get_lighter_darker_colour((10, 10, 10), 50);
    let darker = get_lighter_darker_colour((250, 250, 250), 50);

    assert!(lighter.0 > 10);
    assert!(darker.0 < 250);
}

#[test]
fn text_pixel_helpers_are_inverse() {
    let width = convert_text_to_pixel_width(10.0, 20.0);

    assert_eq!(convert_pixels_to_text_width(10.0, width, false), 20);
}

#[test]
fn mnemonics_are_escaped() {
    assert_eq!(escape_mnemonics("a & b"), "a && b");
}

#[test]
fn tool_tips_wrap_on_word_boundaries() {
    assert_eq!(
        wrap_tool_tip("one two three four", 8),
        "one two\nthree\nfour"
    );
}

#[test]
fn pixmap_scaling_respects_aspect_and_exact_modes() {
    let image = image::DynamicImage::new_rgba8(100, 50);

    let keep = scale_pixmap(&image, 40, 40, true);
    assert_eq!((keep.width(), keep.height()), (40, 20));

    let exact = scale_pixmap(&image, 40, 40, false);
    assert_eq!((exact.width(), exact.height()), (40, 40));
}

#[test]
fn rgba_bytes_round_trip() {
    let image = image::DynamicImage::new_rgba8(2, 2);
    let bytes = image_to_rgba_bytes(&image);

    assert_eq!(bytes.len(), 16);
    assert!(image_from_rgba_bytes(2, 2, &bytes).is_some());
    assert!(image_from_rgba_bytes(2, 2, &bytes[..8]).is_none());
}
