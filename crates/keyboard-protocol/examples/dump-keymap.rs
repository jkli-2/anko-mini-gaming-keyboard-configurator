use std::env;
use std::error::Error;
use std::process::ExitCode;

use hidapi::HidApi;
use keyboard_core::KeyBank;
use keyboard_protocol::{KeyboardDevice, encode_key_action, physical_key};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let bank_name = env::args().nth(1).ok_or("usage: dump-keymap <base|fn>")?;
    let bank = match bank_name.as_str() {
        "base" => KeyBank::Base,
        "fn" => KeyBank::Fn,
        _ => return Err("bank must be 'base' or 'fn'".into()),
    };

    let api = HidApi::new()?;
    let keyboard = KeyboardDevice::open(&api)?;
    let actions = keyboard.read_keymap(bank)?;

    println!("bank: {bank_name}");
    println!("index  key              record       decoded");
    for (index, action) in actions.iter().enumerate() {
        let key = physical_key(index as u16)
            .map(|key| key.id)
            .unwrap_or("<unused>");
        let [kind, code1, code2, code3] = encode_key_action(*action);
        println!(
            "{index:>3}    {key:<16} {kind:02X} {code1:02X} {code2:02X} {code3:02X}  {action:?}"
        );
    }
    Ok(())
}
