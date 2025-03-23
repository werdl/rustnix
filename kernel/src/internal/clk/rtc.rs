use x86_64::instructions::port::Port;

const CMOS_ADDRESS: u16 = 0x70;
const CMOS_DATA: u16 = 0x71;

fn read_rtc_register(register: u8) -> u8 {
    unsafe {
        let mut address_port = Port::new(CMOS_ADDRESS);
        let mut data_port = Port::new(CMOS_DATA);

        address_port.write(register);
        data_port.read()
    }
}

fn bcd_to_binary(value: u8) -> u8 {
    (value & 0x0F) + ((value / 16) * 10)
}

/// Read the current date and time from the RTC
pub fn read_rtc() -> (u8, u8, u8, u8, u8, u8) {
    let second = read_rtc_register(0x00);
    let minute = read_rtc_register(0x02);
    let hour = read_rtc_register(0x04);
    let day = read_rtc_register(0x07);
    let month = read_rtc_register(0x08);
    let year = read_rtc_register(0x09);

    // Convert BCD to binary if necessary
    let second = bcd_to_binary(second);
    let minute = bcd_to_binary(minute);
    let hour = bcd_to_binary(hour);
    let day = bcd_to_binary(day);
    let month = bcd_to_binary(month);
    let year = bcd_to_binary(year);

    (second, minute, hour, day, month, year)
}
