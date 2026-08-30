use std::error::Error;

use hidapi::HidApi;
use keyboard_protocol::{KeyboardDevice, physical_key};

fn main() -> Result<(), Box<dyn Error>> {
    let api = HidApi::new()?;
    let keyboard = KeyboardDevice::open(&api)?;
    let colors = keyboard.read_colors()?;

    println!("index  key              rgb");
    for (index, color) in colors.iter().enumerate() {
        let key = physical_key(index as u16)
            .map(|key| key.id)
            .unwrap_or("<unused>");
        println!(
            "{index:>3}    {key:<16} #{:02X}{:02X}{:02X}",
            color.r, color.g, color.b
        );
    }
    Ok(())
}
