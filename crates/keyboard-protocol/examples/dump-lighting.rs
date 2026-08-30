use std::error::Error;

use hidapi::HidApi;
use keyboard_protocol::KeyboardDevice;

fn main() -> Result<(), Box<dyn Error>> {
    let api = HidApi::new()?;
    let keyboard = KeyboardDevice::open(&api)?;
    let lighting = keyboard.read_lighting()?;

    println!("kind:               {}", lighting.kind);
    println!("effect:             {}", lighting.effect);
    println!("brightness:         {}", lighting.brightness);
    println!("speed:              {}", lighting.speed);
    println!("direction:          {}", lighting.direction);
    println!("color enabled:      {}", lighting.color_enabled);
    println!(
        "hsv bytes:          {}, {}, {}",
        lighting.hsv.hue, lighting.hsv.saturation, lighting.hsv.value
    );

    Ok(())
}
