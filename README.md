# max7219-dot-matrix
Rust driver for the max7219 connected to one or more 8x8 dot matrix led chips

The example below demonstrates how you would pass an instance of MAX7219 around and how you would handle errors

```rust
#![no_std]
#![no_main]

extern crate panic_itm;
use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::{spi::FullDuplex, spi::Mode, spi::Phase, spi::Polarity};
use max7219::{Command, MAX7219};

// bluepill
use stm32f1xx_hal::prelude::*;
use stm32f1xx_hal::spi::Spi;

#[entry]
fn main() -> ! {
    // boilerplate embedded_hal device setup
    let dp = stm32f1xx_hal::stm32::Peripherals::take().unwrap();
    let mut rcc = dp.RCC.constrain();
    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);
    let mut flash = dp.FLASH.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
    let sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
    let miso = gpioa.pa6;
    let mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);
    let mut cs = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);
    let mut spi = Spi::spi1(
        dp.SPI1,
        (sck, miso, mosi),
        &mut afio.mapr,
        Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        },
        2.mhz(), // up to 10mhz for max7219 module, 2mhz is max for bluepill
        clocks,
        &mut rcc.apb2,
    );

    // max 7219 setup for 20 chips
    let mut max7219 = MAX7219::new(&mut cs, 20);

    // use the driver with error handling
    demo_print_string(&mut max7219, &mut spi).unwrap();

    loop {
    }
}

fn demo_print_string<SpiError, PinError, CS>(
    max7219: &mut MAX7219<CS>,
    spi: &mut dyn FullDuplex<u8, Error = SpiError>,
) -> Result<(), max7219::Error<SpiError, PinError>>
    where
        CS: OutputPin<Error = PinError>,
{
    // put the chips in the correct display state (need to do this once on startup)
    max7219.write_command_all(spi, Command::OnOff, 0)?;
    max7219.write_command_all(spi, Command::ScanLimit, 7)?;
    max7219.write_command_all(spi, Command::DecodeMode, 0)?;
    max7219.write_command_all(spi, Command::DisplayTest, 0)?;
    max7219.write_command_all(spi, Command::Intensity, 1)?;
    max7219.clear_all(spi)?;
    max7219.write_command_all(spi, Command::OnOff, 1)?;

    // write out a string at position 0 (position can be negative)
    max7219.write_str_at_pos(spi, "Hello, World!", 0)?;
    Ok(())
}
```

See https://github.com/ninjasource/led-display-websocket-demo for a full demo