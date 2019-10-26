fn is_bit_set (byte : u8, n : u8) -> bool {
    if n < 8 {
        byte & (1 << n) != 0
    } else {
        false
    }
}

/// Rotate an 8x8 matrix by 90 degrees clockwise
/// used in this project for rotating letters in a font file
pub fn rotate_90_clockwise (buffer: [u8; 8]) -> [u8; 8]{
    let mut rotated: [u8; 8] = [0; 8];

    for (i, line) in buffer.iter().enumerate() {
        for j in 0..8 {
            if is_bit_set(*line, j) {
                let mask: u8 = 1 << i as u8;
                rotated[7 - j] = rotated[7 - j] | mask;
            }
        }
    }

    rotated
}