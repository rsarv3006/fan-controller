#![no_std]
#![no_main]

use panic_halt as _;

enum Command {
    SetSpeed(u8),
    GetTemp
}

fn read_dht22_temp() -> i16 {
    return 732
}

fn set_pwm(duty: u8) {}

fn read_serial_command(serial: &mut impl ufmt::uWrite) -> Option<Command> {
    return None
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    let mut led = pins.d13.into_output();

    let mut loop_count: u8 = 0;

    loop {
        loop_count += 1;

        if let Some(command) = read_serial_command(&mut serial) {
            match command {
                SetSpeed(duty) => {

                }
                GetTemp => {
                    let temp = read_dht22_temp();
                }
            }
        }

        if loop_count >= 30 {
            loop_count = 0;
            let temp = read_dht22_temp();
            ufmt::uwriteln!(&mut serial, "TEMP:{}", temp).ok();
        }
    }
}
