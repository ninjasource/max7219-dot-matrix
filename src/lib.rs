#![no_std]

// This is a driver for the MAX7219 connected to a 8x8 LED dot matrix display. It supports multiple
// daisy-chained displays. Not to be confused with other rust MAX7219 drivers which are designed for
// use with a standard 7 segment LED display.
// see http://www.gammon.com.au/forum/?id=11516 a description of this chip and uses
// see also https://github.com/nickgammon/MAX7219

extern crate embedded_hal;
use embedded_hal::digital::v2::OutputPin;
mod font;
use embedded_hal::spi::FullDuplex;
use font::*;

#[macro_use(block)]
extern crate nb;

pub enum Command {
    Noop = 0x00,
    Digit0 = 0x01,
    Digit1 = 0x02,
    Digit2 = 0x03,
    Digit3 = 0x04,
    Digit4 = 0x05,
    Digit5 = 0x06,
    Digit6 = 0x07,
    Digit7 = 0x08,
    DecodeMode = 0x09,
    Intensity = 0x0A,
    ScanLimit = 0x0B,
    OnOff = 0x0C,
    DisplayTest = 0x0F,
}

pub struct MAX7219<'a, CS> {
    cs: &'a mut CS,
    num_devices: u8,
}

impl<'a, CS, PinError> MAX7219<'a, CS>
where
    CS: OutputPin<Error = PinError>,
{
    pub fn new(cs: &'a mut CS, num_devices: u8) -> Self {
        MAX7219 { cs, num_devices }
    }

    // write command to all chips
    pub fn write_command_all<E>(
        &mut self,
        spi: &mut FullDuplex<u8, Error = E>,
        command: Command,
        data: u8,
    ) {
        self.write_raw_all(spi, command as u8, data);
    }

    // write raw bytes to all chips
    pub fn write_raw_all<E>(
        &mut self,
        spi: &mut FullDuplex<u8, Error = E>,
        register: u8,
        data: u8,
    ) {
        self.cs.set_low();
        for _ in 0..self.num_devices {
            self.shift_out(spi, register);
            self.shift_out(spi, data);
        }
        self.cs.set_high();
    }

    // write a single byte to a chip a certain position where zero is the first chip
    // this supports daisy chaining multiple chips together.
    fn write_raw_spi<E>(
        &mut self,
        spi: &mut FullDuplex<u8, Error = E>,
        position: usize,
        register: u8,
        data: u8,
    ) {
        self.cs.set_low();

        // skip MAX7219 chips after the text (yes, after)
        for _ in position..self.num_devices - 1 {
            self.shift_out(spi, 0);
            self.shift_out(spi, 0);
        }

        // write one line
        self.shift_out(spi, register);
        self.shift_out(spi, data);

        // skip MAX7219 chips before text
        for _ in 0..position {
            self.shift_out(spi, 0);
            self.shift_out(spi, 0);
        }

        self.cs.set_high();
    }

    // use this function to write a string to the led display starting at position zero
    pub fn write_str<E>(&mut self, spi: &mut FullDuplex<u8, Error = E>, s: &str) {
        for (string_index, font_index) in s.as_bytes().iter().enumerate() {

            // get a single character bitmap (8x8) from the font array
            let buffer = CP437FONT[*font_index as usize];

            // write each line of the font to a corresponding register on the max7219
            // at the position offset specified
            for (i, line) in buffer.iter().enumerate() {
                let register = (i + 1) as u8;
                self.write_raw_spi(spi, string_index, register, *line);
            }
        }
    }

    // use this nightmare function to text to the led display at an arbitrary position.
    // primarily used for scrolling text
    // x is the pixel position in the horizontal direction and can be negative
    pub fn write_str_at_pos<E>(&mut self, spi: &mut FullDuplex<u8, Error = E>, s: &str, x: i32) {
        let string = s.as_bytes();
        let abs_x = if x < 0 { -x } else { x };
        let shift_by_bits = (abs_x % 8) as u8;
        let start_string_index = x / 8;

        for (string_index, font_index) in string.iter().enumerate() {
            let position = string_index as i32 + start_string_index;
            if position >= 0 {
                let left = if string_index > 0 {
                    CP437FONT[string[string_index - 1] as usize]
                } else {
                    CP437FONT[0]
                };
                let middle = CP437FONT[string[string_index] as usize];
                let right = if string_index < string.len() - 1 {
                    CP437FONT[string[string_index + 1] as usize]
                } else {
                    CP437FONT[0]
                };

                for i in 0..8 {
                    let register = (i + 1) as u8;

                    let val = if shift_by_bits == 0 {
                        middle[i]
                    } else if x < 0 {
                        // shift digit left
                        middle[i] >> shift_by_bits ^ right[i] << (8 - shift_by_bits)
                    } else {
                        // shift digit right
                        middle[i] << shift_by_bits ^ left[i] >> (8 - shift_by_bits)
                    };

                    self.write_raw_spi(spi, position as u8, register, val);
                }
            }
        }

        // add the remainder of the last character onto the end
        if x >= 0 && shift_by_bits != 0 {
            let string_index = string.len() - 1;
            let position = string_index as i32 + start_string_index + 1;
            if position >= 0 {
                let middle = CP437FONT[string[string_index] as usize];
                for i in 0..8 {
                    let register = (i + 1) as u8;
                    let val = middle[i] >> (8 - shift_by_bits);
                    self.write_raw_spi(spi, position as u8, register, val);
                }
            }
        }
    }

    fn shift_out<E>(&mut self, spi: &mut FullDuplex<u8, Error = E>, value: u8) {
        block!(spi.send(value));
        block!(spi.read());
    }
}
