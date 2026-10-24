//! Rust port of `hydrus/core/files/images/HydrusImageMathSlop.py`.
//!
//! The Python module uses numpy for a handful of 3x3 colour-science
//! operations. This port keeps them in plain arrays so the core crate does not
//! need a linear-algebra dependency.

/// A row-major 3x3 matrix.
pub type Matrix3 = [[f64; 3]; 3];

/// Default D50 white point in xy chromaticity coordinates.
pub const D50_XY: (f64, f64) = (0.34567, 0.35850);

const BRADFORD: Matrix3 = [
    [0.8951, 0.2664, -0.1614],
    [-0.7502, 1.7135, 0.0367],
    [0.0389, -0.0685, 1.0296],
];

pub fn mat_mul(a: Matrix3, b: Matrix3) -> Matrix3 {
    let mut result = [[0.0; 3]; 3];

    for row in 0..3 {
        for column in 0..3 {
            result[row][column] = (0..3).map(|k| a[row][k] * b[k][column]).sum();
        }
    }

    result
}

pub fn mat_vec_mul(matrix: Matrix3, vector: [f64; 3]) -> [f64; 3] {
    let mut result = [0.0; 3];

    for row in 0..3 {
        result[row] = (0..3)
            .map(|column| matrix[row][column] * vector[column])
            .sum();
    }

    result
}

/// Invert a 3x3 matrix with Gauss-Jordan elimination.
///
/// Returns `None` when the matrix is singular. Python's numpy raises in that
/// case; callers in this crate turn that into a colour-parsing failure.
pub fn mat_inverse(matrix: Matrix3) -> Option<Matrix3> {
    let mut augmented = [[0.0; 6]; 3];

    for row in 0..3 {
        for column in 0..3 {
            augmented[row][column] = matrix[row][column];
        }

        augmented[row][row + 3] = 1.0;
    }

    for pivot_column in 0..3 {
        let pivot_row = (pivot_column..3).max_by(|left, right| {
            augmented[*left][pivot_column]
                .abs()
                .partial_cmp(&augmented[*right][pivot_column].abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;

        if augmented[pivot_row][pivot_column].abs() < f64::EPSILON {
            return None;
        }

        augmented.swap(pivot_column, pivot_row);

        let pivot = augmented[pivot_column][pivot_column];

        for value in &mut augmented[pivot_column] {
            *value /= pivot;
        }

        for row in 0..3 {
            if row == pivot_column {
                continue;
            }

            let factor = augmented[row][pivot_column];

            for column in 0..6 {
                augmented[row][column] -= factor * augmented[pivot_column][column];
            }
        }
    }

    let mut inverse = [[0.0; 3]; 3];

    for row in 0..3 {
        for column in 0..3 {
            inverse[row][column] = augmented[row][column + 3];
        }
    }

    Some(inverse)
}

/// `xy_to_xyz( x, y )`
pub fn xy_to_xyz(x: f64, y: f64) -> [f64; 3] {
    [x / y, 1.0, (1.0 - x - y) / y]
}

/// `float_to_s15Fixed16( val )`
pub fn float_to_s15_fixed16(value: f64) -> i32 {
    (value * 65536.0).round() as i32
}

/// `generate_bradford_adaptation_matrix( src_xy, dst_xy = D50 )`
pub fn generate_bradford_adaptation_matrix(src_xy: (f64, f64)) -> Matrix3 {
    generate_bradford_adaptation_matrix_to(src_xy, D50_XY)
}

/// `generate_bradford_adaptation_matrix( src_xy, dst_xy )`
pub fn generate_bradford_adaptation_matrix_to(src_xy: (f64, f64), dst_xy: (f64, f64)) -> Matrix3 {
    let src_xyz = xy_to_xyz(src_xy.0, src_xy.1);
    let dst_xyz = xy_to_xyz(dst_xy.0, dst_xy.1);

    let src_cone = mat_vec_mul(BRADFORD, src_xyz);
    let dst_cone = mat_vec_mul(BRADFORD, dst_xyz);

    let scale = [
        [dst_cone[0] / src_cone[0], 0.0, 0.0],
        [0.0, dst_cone[1] / src_cone[1], 0.0],
        [0.0, 0.0, dst_cone[2] / src_cone[2]],
    ];

    let bradford_inverse = mat_inverse(BRADFORD).expect("the Bradford matrix is invertible");

    mat_mul(mat_mul(bradford_inverse, scale), BRADFORD)
}

/// `adapt_rgb_to_xyz_matrix_to_D50( m_rgb_to_xyz, src_white_xy )`
pub fn adapt_rgb_to_xyz_matrix_to_d50(rgb_to_xyz: Matrix3, src_white_xy: (f64, f64)) -> Matrix3 {
    let adaptation = generate_bradford_adaptation_matrix(src_white_xy);

    mat_mul(adaptation, rgb_to_xyz)
}

/// `adapt_white_point( XYZ_src_white, XYZ_dst_white )`
pub fn adapt_white_point(xyz_src_white: [f64; 3], xyz_dst_white: [f64; 3]) -> Matrix3 {
    let src_cone = mat_vec_mul(BRADFORD, xyz_src_white);
    let dst_cone = mat_vec_mul(BRADFORD, xyz_dst_white);

    let scale = [
        [dst_cone[0] / src_cone[0], 0.0, 0.0],
        [0.0, dst_cone[1] / src_cone[1], 0.0],
        [0.0, 0.0, dst_cone[2] / src_cone[2]],
    ];

    let bradford_inverse = mat_inverse(BRADFORD).expect("the Bradford matrix is invertible");

    mat_mul(mat_mul(bradford_inverse, scale), BRADFORD)
}

/// `chromaticities_to_rgb_to_xyz( white, red, green, blue )`
pub fn chromaticities_to_rgb_to_xyz(
    white: (f64, f64),
    red: (f64, f64),
    green: (f64, f64),
    blue: (f64, f64),
) -> Matrix3 {
    let white_xyz = xy_to_xyz(white.0, white.1);
    let red_xyz = xy_to_xyz(red.0, red.1);
    let green_xyz = xy_to_xyz(green.0, green.1);
    let blue_xyz = xy_to_xyz(blue.0, blue.1);

    // Columns are R, G, B.
    let unscaled = [
        [red_xyz[0], green_xyz[0], blue_xyz[0]],
        [red_xyz[1], green_xyz[1], blue_xyz[1]],
        [red_xyz[2], green_xyz[2], blue_xyz[2]],
    ];

    let inverse = mat_inverse(unscaled).expect("the RGB primaries are independent");
    let scale = mat_vec_mul(inverse, white_xyz);

    let mut result = unscaled;

    for row in 0..3 {
        for column in 0..3 {
            result[row][column] *= scale[column];
        }
    }

    result
}

/// `srgb_encode( c )`
pub fn srgb_encode(value: f64) -> f64 {
    let a = 0.055;

    if value <= 0.0031308 {
        12.92 * value
    } else {
        (1.0 + a) * value.powf(1.0 / 2.4) - a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_matrix_close(actual: Matrix3, expected: Matrix3) {
        for row in 0..3 {
            for column in 0..3 {
                assert!(
                    (actual[row][column] - expected[row][column]).abs() < 1e-4,
                    "({row}, {column}): {} != {}",
                    actual[row][column],
                    expected[row][column]
                );
            }
        }
    }

    #[test]
    fn inverse_round_trips() {
        let matrix = [[4.0, 7.0, 2.0], [3.0, 6.0, 1.0], [2.0, 5.0, 3.0]];
        let inverse = mat_inverse(matrix).unwrap();

        assert_matrix_close(
            mat_mul(matrix, inverse),
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        );
    }

    #[test]
    fn d65_to_d50_is_the_known_bradford_matrix() {
        let actual = generate_bradford_adaptation_matrix_to((0.3127, 0.3290), D50_XY);

        assert_matrix_close(
            actual,
            [
                [1.047884, 0.022925, -0.050217],
                [0.029604, 0.990484, -0.017076],
                [-0.009245, 0.015052, 0.751880],
            ],
        );
    }

    #[test]
    fn chromaticities_make_a_white_preserving_matrix() {
        let srgb = chromaticities_to_rgb_to_xyz(
            (0.3127, 0.3290),
            (0.6400, 0.3300),
            (0.3000, 0.6000),
            (0.1500, 0.0600),
        );

        let white = mat_vec_mul(srgb, [1.0, 1.0, 1.0]);

        let expected = xy_to_xyz(0.3127, 0.3290);

        for channel in 0..3 {
            assert!((white[channel] - expected[channel]).abs() < 1e-9);
        }
    }

    #[test]
    fn fixed_point_and_srgb_helpers_match_the_python() {
        assert_eq!(float_to_s15_fixed16(1.0), 65536);
        assert!((srgb_encode(0.003) - 0.03876).abs() < 1e-9);
    }
}
