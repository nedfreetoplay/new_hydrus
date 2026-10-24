//! Colour conversion helpers.

/// `srgb_to_linear( value )`
pub fn srgb_to_linear(value: f64) -> f64 {
    let value = value / 255.0;

    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// `sign_pow( value, exp )`
pub fn sign_pow(value: f64, exponent: f64) -> f64 {
    value.abs().powf(exponent).copysign(value)
}

/// `linear_to_srgb( value )`
pub fn linear_to_srgb(value: f64) -> u8 {
    let value = value.clamp(0.0, 1.0);

    let converted = if value <= 0.0031308 {
        value * 12.92
    } else {
        (1.055 * value.powf(1.0 / 2.4)) - 0.055
    };

    (converted * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}
