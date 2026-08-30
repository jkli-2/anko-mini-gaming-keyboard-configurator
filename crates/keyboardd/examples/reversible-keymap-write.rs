use std::error::Error;

use keyboardd::{INTERFACE_NAME, OBJECT_PATH, SERVICE_NAME};
use zbus::blocking::{Connection, Proxy};

type Assignments = Vec<(String, String)>;

fn get_keymap(proxy: &Proxy<'_>) -> zbus::Result<Assignments> {
    proxy.call("GetKeymap", &("base",))
}

fn set_key(proxy: &Proxy<'_>, action: &str) -> zbus::Result<()> {
    proxy.call("SetKey", &("base", "esc", action))
}

fn set_keymap(proxy: &Proxy<'_>, assignments: &Assignments) -> zbus::Result<()> {
    proxy.call("SetKeymap", &("base", assignments))
}

fn esc_action(assignments: &Assignments) -> Result<&str, Box<dyn Error>> {
    assignments
        .iter()
        .find(|(id, _)| id == "esc")
        .map(|(_, action)| action.as_str())
        .ok_or_else(|| "GetKeymap omitted the esc key".into())
}

fn main() -> Result<(), Box<dyn Error>> {
    eprintln!("WARNING: this diagnostic temporarily writes the Base Esc assignment.");
    let connection = Connection::session()?;
    let proxy = Proxy::new(&connection, SERVICE_NAME, OBJECT_PATH, INTERFACE_NAME)?;
    let original = get_keymap(&proxy)?;
    let original_esc = esc_action(&original)?.to_string();
    let temporary = if original_esc == "keyboard/69/0" {
        "keyboard/68/0"
    } else {
        "keyboard/69/0"
    };

    let single_result = (|| -> Result<(), Box<dyn Error>> {
        set_key(&proxy, temporary)?;
        if esc_action(&get_keymap(&proxy)?)? != temporary {
            return Err("single-key write did not read back".into());
        }
        Ok(())
    })();
    let single_restore = (|| -> Result<(), Box<dyn Error>> {
        set_key(&proxy, &original_esc)?;
        if esc_action(&get_keymap(&proxy)?)? != original_esc {
            return Err("single-key restoration did not read back".into());
        }
        Ok(())
    })();
    if let Err(error) = single_restore {
        return Err(format!("single-key restoration failed: {error}").into());
    }
    single_result?;
    println!("SetKey change, readback, restore, and final readback succeeded.");

    let mut changed = original.clone();
    changed
        .iter_mut()
        .find(|(id, _)| id == "esc")
        .ok_or("GetKeymap omitted the esc key")?
        .1 = temporary.to_string();

    let bulk_result = (|| -> Result<(), Box<dyn Error>> {
        set_keymap(&proxy, &changed)?;
        if get_keymap(&proxy)? != changed {
            return Err("bulk keymap write did not read back exactly".into());
        }
        Ok(())
    })();
    let bulk_restore = (|| -> Result<(), Box<dyn Error>> {
        set_keymap(&proxy, &original)?;
        if get_keymap(&proxy)? != original {
            return Err("bulk keymap restoration did not read back exactly".into());
        }
        Ok(())
    })();
    if let Err(error) = bulk_restore {
        return Err(format!("bulk keymap restoration failed: {error}").into());
    }
    bulk_result?;
    println!("SetKeymap change, readback, restore, and final readback succeeded.");
    Ok(())
}
