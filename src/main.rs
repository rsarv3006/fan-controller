#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::port::{Pin, mode};

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    
    let mut dht_pin = pins.d2.into_floating_input();

    loop {
        let (new_pin, result) = read_dht22(dht_pin);
        dht_pin = new_pin;

        match result {
            Some((t, h)) => ufmt::uwriteln!(&mut serial, "MSG:T:{}-H:{}", t, h).ok(),
            None => ufmt::uwriteln!(&mut serial, "ERR:TEMP_READ_FAIL").ok(),
        };

    arduino_hal::delay_ms(3000);
}

}

fn read_dht22<PIN>(
    pin: Pin<mode::Input<mode::Floating>, PIN>
) -> (Pin<mode::Input<mode::Floating>, PIN>, Option<(i16, i16)>)
where
    PIN: arduino_hal::hal::port::PinOps,
{
    let mut data = [0u8; 5];

    let mut pin = pin.into_output();
    pin.set_low();
    arduino_hal::delay_ms(1);

    pin.set_high();
    arduino_hal::delay_us(30);

    let pin = pin.into_floating_input();

    // Wait for DHT response
    let mut timeout = 0u16;
    while pin.is_high() {
        arduino_hal::delay_us(1);
        timeout += 1;
        if timeout > 100 {
            return (pin, None);
        }
    }

    timeout = 0;
    while pin.is_low() {
        arduino_hal::delay_us(1);
        timeout += 1;
        if timeout > 100 {
            return (pin, None);
        }
    }

    timeout = 0;
    while pin.is_high() {
        arduino_hal::delay_us(1);
        timeout += 1;
        if timeout > 100 {
            return (pin, None);
        }
    }

    // --- Step 3: read 40 bits ---
    for b in 0..5 {
        for i in 0..8 {
            timeout = 0;
            while pin.is_low() {
                arduino_hal::delay_us(1);
                timeout += 1;
                if timeout > 70 {
                    return (pin, None);
                }
            }

            arduino_hal::delay_us(40);

            if pin.is_high() {
                data[b] |= 1 << (7 - i);
            }

            timeout = 0;
            while pin.is_high() {
                arduino_hal::delay_us(1);
                timeout += 1;
                if timeout > 100 {
                    return (pin, None);
                }
            }
        }
    }

    // pin already in floating input mode — return it

    // --- Step 4: checksum ---
    let cs = data[0].wrapping_add(data[1])
        .wrapping_add(data[2])
        .wrapping_add(data[3]);
    if cs != data[4] {
        return (pin, None);
    }

    // --- Step 5: decode ---
    let humidity = ((data[0] as u16) << 8 | data[1] as u16) as i16;
    let mut temperature = ((data[2] as u16) << 8 | data[3] as u16) as i16;

    if (data[2] & 0x80) != 0 {
        temperature = -(temperature & 0x7FFF);
    }

    (pin, Some((temperature, humidity)))
}


