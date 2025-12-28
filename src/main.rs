#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::port::{Pin, mode};
use arduino_hal::prelude::*;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);
    
    // DHT22 on pin 2
    let mut dht_pin = pins.d2.into_floating_input();
    
    // Fan PWM control on pin 9
    let mut _fan_pwm = pins.d9.into_output();
    
    // Configure Timer1 for 25kHz PWM on pin 9
    setup_timer1_pwm(&dp.TC1);
    
    // Default fan speed: 50%
    let mut fan_speed: u8 = 50;
    set_fan_speed(&dp.TC1, fan_speed);
    
    ufmt::uwriteln!(&mut serial, "Fan controller started, default 50%").ok();
    
    let mut buffer = [0u8; 32];
    let mut buf_idx = 0;
    
    loop {
        // Read temperature and humidity
        let (new_pin, result) = read_dht22(dht_pin);
        dht_pin = new_pin;
        
        match result {
            Some((t, h)) => {
                ufmt::uwriteln!(&mut serial, "MSG:T:{}-H:{}", t, h).ok();
            }
            None => {
                ufmt::uwriteln!(&mut serial, "ERR:TEMP_READ_FAIL").ok();
            }
        };
        
        // Check for incoming serial commands (non-blocking)
        // Expected format: "FAN:75" to set 75% speed
        for _ in 0..100 {
            match nb::block!(serial.read()) {
                Ok(byte) => {
                    if byte == b'\n' || byte == b'\r' {
                        // Parse command
                        if let Some(speed) = parse_fan_command(&buffer[..buf_idx]) {
                            fan_speed = speed;
                            set_fan_speed(&dp.TC1, fan_speed);
                            ufmt::uwriteln!(&mut serial, "FAN:{}", fan_speed).ok();
                        }
                        buf_idx = 0;
                    } else if buf_idx < buffer.len() {
                        buffer[buf_idx] = byte;
                        buf_idx += 1;
                    }
                }
                Err(_) => break, // No more data available
            }
        }
        
        arduino_hal::delay_ms(9900); // Complete the 10 second cycle
    }
}

// Parse "FAN:XX" command, returns speed 0-100
fn parse_fan_command(buf: &[u8]) -> Option<u8> {
    // Look for "FAN:" prefix
    if buf.len() < 5 || &buf[..4] != b"FAN:" {
        return None;
    }
    
    // Parse number after "FAN:"
    let mut speed: u16 = 0;
    for &byte in &buf[4..] {
        if byte >= b'0' && byte <= b'9' {
            speed = speed * 10 + (byte - b'0') as u16;
        } else {
            break;
        }
    }
    
    if speed > 100 {
        Some(100)
    } else {
        Some(speed as u8)
    }
}

// Setup Timer1 for 25kHz PWM (standard for PC fans)
fn setup_timer1_pwm(tc1: &arduino_hal::pac::TC1) {
    tc1.tccr1a().write(|w| unsafe {
        w.com1a().match_clear()
         .wgm1().bits(0b10)
    });
    
    tc1.tccr1b().write(|w| unsafe {
        w.wgm1().bits(0b11)
         .cs1().direct()
    });
    
    // Set frequency to 25kHz: 16MHz / 25kHz = 640
    tc1.icr1().write(|w| unsafe { w.bits(640) });
    
    // Start with fan off
    tc1.ocr1a().write(|w| unsafe { w.bits(0) });
}

// Set fan speed (0-100%)
fn set_fan_speed(tc1: &arduino_hal::pac::TC1, speed: u8) {
    let speed = if speed > 100 { 100 } else { speed };
    
    // Calculate duty cycle
    let duty = (640u32 * speed as u32) / 100;
    
    tc1.ocr1a().write(|w| unsafe { w.bits(duty as u16) });
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
    
    // Read 40 bits
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
    
    // Checksum
    let cs = data[0].wrapping_add(data[1])
        .wrapping_add(data[2])
        .wrapping_add(data[3]);
    if cs != data[4] {
        return (pin, None);
    }
    
    // Decode
    let humidity = ((data[0] as u16) << 8 | data[1] as u16) as i16;
    let mut temperature = ((data[2] as u16) << 8 | data[3] as u16) as i16;
    if (data[2] & 0x80) != 0 {
        temperature = -(temperature & 0x7FFF);
    }
    
    (pin, Some((temperature, humidity)))
}
