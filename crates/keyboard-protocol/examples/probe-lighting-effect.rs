use std::error::Error;

use hidapi::HidApi;
use keyboard_protocol::KeyboardDevice;

fn main() -> Result<(), Box<dyn Error>> {
    let effect = std::env::args()
        .nth(1)
        .ok_or("usage: probe-lighting-effect <effect-id>")?
        .parse::<u8>()?;

    let api = HidApi::new()?;
    let keyboard = KeyboardDevice::open(&api)?;
    let before = keyboard.read_lighting()?;
    let candidate = keyboard.read_lighting_effect_config(effect)?;
    let after = keyboard.read_lighting()?;

    println!("active before: {before:?}");
    println!("effect {effect} config: {candidate:?}");
    println!("active after:  {after:?}");

    if before != after {
        return Err("06 16 changed active lighting; stop probing".into());
    }
    Ok(())
}
