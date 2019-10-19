#![no_std]

// This is a driver for the MAX7219 connected to a 8x8 LED dot matrix display. It supports multiple
// daisy-chained displays. Not to be confused with other rust MAX7219 drivers which are designed for
// use with a standard 7 segment LED display.
// see http://www.gammon.com.au/forum/?id=11516 a description of this chip and uses
// see also https://github.com/nickgammon/MAX7219

extern crate embedded_hal;
use embedded_hal::digital::v2::OutputPin;
mod font;
use font::*;

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

pub struct MAX7219<DATA, CS, CLK> {
    data: DATA,
    cs: CS,
    clk: CLK,
    num_devices: u8,
}

impl<DATA, CS, CLK, PinError> MAX7219<DATA, CS, CLK>
where
    DATA: OutputPin<Error = PinError>,
    CS: OutputPin<Error = PinError>,
    CLK: OutputPin<Error = PinError>,
{
    pub fn new(data: DATA, cs: CS, clk: CLK, num_devices: u8) -> Self {
        let mut max7219 = MAX7219 {
            data,
            cs,
            clk,
            num_devices,
        };

        max7219
    }

    pub fn write_command_all(&mut self, command: Command, data: u8) {
        self.write_raw_all(command as u8, data);
    }

    fn write_raw(&mut self, position: u8, register: u8, data: u8) {
        self.cs.set_low();

        // write blank cells after text (yes, after)
        for _ in position..self.num_devices - 1 {
            self.shift_out(0);
            self.shift_out(0);
        }

        self.shift_out(register);
        self.shift_out(data);

        // write blank cells before text
        for _ in 0..position {
            self.shift_out(0);
            self.shift_out(0);
        }

        self.cs.set_high();
    }

    pub fn write_raw_all(&mut self, register: u8, data: u8) {
        self.cs.set_low();
        for _ in 0..self.num_devices {
            self.shift_out(register);
            self.shift_out(data);
        }
        self.cs.set_high();
    }

    pub fn write_str(&mut self, s: &str) {
        for (string_index, font_index) in s.as_bytes().iter().enumerate() {
            let buffer = CP437FONT[*font_index as usize];

            for (i, line) in buffer.iter().enumerate() {
                let register = (i + 1) as u8;
                self.write_raw(string_index as u8, register, *line);
            }
        }
    }

    fn shift_out(&mut self, value: u8) {
        for i in 0..8 {
            if value & (1 << (7 - i)) > 0 {
                self.data.set_high();
            } else {
                self.data.set_low();
            }

            self.clk.set_high();
            self.clk.set_low();
        }
    }
}
