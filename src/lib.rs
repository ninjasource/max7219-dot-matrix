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
    num_devices: usize,
}

impl<'a, CS, PinError> MAX7219<'a, CS>
where
    CS: OutputPin<Error = PinError>,
{
    pub fn new(cs: &'a mut CS, num_devices: usize) -> Self {
        MAX7219 { cs, num_devices }
    }

    pub fn get_num_devices(&mut self) -> usize {
        self.num_devices
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

    /// clear the display
    pub fn clear_all<E>(&mut self, spi: &mut FullDuplex<u8, Error = E>) {
        for register in 1..9 {
            self.cs.set_low();
            for _ in 0..self.num_devices {
                self.shift_out(spi, register);
                self.shift_out(spi, 0);
            }
            self.cs.set_high();
        }
    }

    // write the same raw byte to all chips
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

    /// payload should have num_devices number of bytes in it
    /// line_index should be between 0 and 7 (bottom to top if the led serial number is facing up)
    pub fn write_line_raw<E>(
        &mut self,
        spi: &mut FullDuplex<u8, Error = E>,
        line_index: u8,
        payload: &[u8],
    ) {
        if line_index >= 0 && line_index < 8 && payload.len() == self.num_devices {
            self.cs.set_low();
            let register = line_index + 1;
            for data in payload {
                self.shift_out(spi, register);
                self.shift_out(spi, *data);
            }
            self.cs.set_high();
        }
    }

    // write raw bytes to all chips
    fn push_raw<E>(&mut self, spi: &mut FullDuplex<u8, Error = E>, register: u8, data: u8) {
        self.shift_out(spi, register);
        self.shift_out(spi, data);
    }

    // write a single byte to a chip a certain position where zero is the first chip
    // this supports daisy chaining multiple chips together.
    fn write_raw<E>(
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

    fn get_byte_at(
        &mut self,
        string: &[u8],
        string_index: usize,
        line_index: usize,
        shift_by_num_bits: i8,
    ) -> u8 {
        let left_index = string_index as i32 - 1;
        let mid_index = string_index;
        let right_index = string_index + 1;
        let len = string.len() as i32;

        let left = if is_in_range(len, left_index) {
            CP437FONT[string[left_index as usize] as usize]
        } else {
            CP437FONT[0]
        };
        let middle = if is_in_range(len, mid_index as i32) {
            CP437FONT[string[mid_index] as usize]
        } else {
            CP437FONT[0]
        };
        let right = if is_in_range(len, right_index as i32) {
            CP437FONT[string[right_index] as usize]
        } else {
            CP437FONT[0]
        };

        let val = if shift_by_num_bits == 0 {
            middle[line_index]
        } else if shift_by_num_bits < 0 {
            // shift digit left
            let shift_by_num_bits = -shift_by_num_bits as u8;
            middle[line_index] >> shift_by_num_bits ^ right[line_index] << (8 - shift_by_num_bits)
        } else {
            // shift digit right
            let shift_by_num_bits = shift_by_num_bits as u8;
            middle[line_index] << shift_by_num_bits ^ left[line_index] >> (8 - shift_by_num_bits)
        };

        val
    }

    // use this nightmare function to text to the led display at an arbitrary position.
    // primarily used for scrolling text
    // x is the pixel position in the horizontal direction and can be negative
    pub fn write_str_at_pos<E>(
        &mut self,
        spi: &mut FullDuplex<u8, Error = E>,
        s: &str,
        x_pos: i32,
    ) {
        let string = s.as_bytes();
        let shift_by_bits = (x_pos % 8) as i8;
        let start_string_index = x_pos / 8;

        for line_index in 0..8 {
            self.cs.set_low();

            // write one line
            for chip_index in 0..self.num_devices {
                // write the string backwards because we push bytes onto the bus so the last
                // character appears first
                let string_index =
                    self.num_devices as i32 - chip_index as i32 - 1 - start_string_index as i32;
                let register = line_index as u8 + 1;

                // bit of a strange range check here but we need to draw the remainder of the last character
                if string_index >= 0 && string_index <= string.len() as i32 {
                    // we may need to draw a single character over two chips so we need to do some bit shifting
                    let val =
                        self.get_byte_at(string, string_index as usize, line_index, shift_by_bits);
                    self.push_raw(spi, register, val);
                } else {
                    self.push_raw(spi, register, 0);
                }
            }

            // latch
            self.cs.set_high();
        }
    }

    fn shift_out<E>(&mut self, spi: &mut FullDuplex<u8, Error = E>, value: u8) {
        block!(spi.send(value));
        block!(spi.read());
    }
}

fn is_in_range(len: i32, i: i32) -> bool {
    i >= 0 && i < len
}
