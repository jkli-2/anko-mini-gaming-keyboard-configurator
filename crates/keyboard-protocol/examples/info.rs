use std::error::Error;

use hidapi::HidApi;
use keyboard_protocol::read_keyboard_config;

fn main() -> Result<(), Box<dyn Error>> {
    let api = HidApi::new()?;
    let config = read_keyboard_config(&api)?;

    println!("Anko/Kmart Mini Gaming Keyboard");
    println!("PID:       {:04X}", config.product_id);
    println!("Firmware:  {}", config.firmware_version);
    println!("Protocol:  {}", config.protocol_version);
    println!("Layers:    {}", config.layer_count);
    println!("Layer:     {}", config.layer);
    println!("Work mode: {}", config.work_mode);
    println!("Link:      {}", config.link_status);
    if let Some(seconds) = config.auto_sleep_seconds {
        println!("Auto-sleep: {seconds} seconds");
    }
    if let Some(serial) = &config.serial_number {
        println!("Serial:    {serial}");
    }

    Ok(())
}
