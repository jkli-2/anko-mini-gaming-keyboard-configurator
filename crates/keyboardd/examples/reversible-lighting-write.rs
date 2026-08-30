use std::error::Error;

use keyboardd::{INTERFACE_NAME, OBJECT_PATH, SERVICE_NAME};
use zbus::blocking::{Connection, Proxy};

type Lighting = (u8, u8, u8, u8, u8, bool, u8, u8, u8, u8);
type Colors = Vec<(String, u8, u8, u8)>;

fn get_lighting(proxy: &Proxy<'_>) -> zbus::Result<Lighting> {
    proxy.call("GetLighting", &())
}

fn set_lighting(proxy: &Proxy<'_>, state: Lighting) -> zbus::Result<()> {
    proxy.call("SetLighting", &state)
}

fn get_colors(proxy: &Proxy<'_>) -> zbus::Result<Colors> {
    proxy.call("GetKeyColors", &())
}

fn set_color(proxy: &Proxy<'_>, color: (u8, u8, u8)) -> zbus::Result<()> {
    proxy.call("SetKeyColor", &("esc", color.0, color.1, color.2))
}

fn set_colors(proxy: &Proxy<'_>, colors: &Colors) -> zbus::Result<()> {
    proxy.call("SetKeyColors", &(colors,))
}

fn esc_color(colors: &Colors) -> Result<(u8, u8, u8), Box<dyn Error>> {
    colors
        .iter()
        .find(|(id, _, _, _)| id == "esc")
        .map(|(_, red, green, blue)| (*red, *green, *blue))
        .ok_or_else(|| "GetKeyColors omitted the esc key".into())
}

fn main() -> Result<(), Box<dyn Error>> {
    eprintln!("WARNING: this diagnostic temporarily writes lighting and Base Esc RGB state.");
    let connection = Connection::session()?;
    let proxy = Proxy::new(&connection, SERVICE_NAME, OBJECT_PATH, INTERFACE_NAME)?;

    let original_lighting = get_lighting(&proxy)?;
    let mut changed_lighting = original_lighting;
    changed_lighting.1 = if original_lighting.1 == 1 { 2 } else { 1 };
    let lighting_result = (|| -> Result<(), Box<dyn Error>> {
        set_lighting(&proxy, changed_lighting)?;
        if get_lighting(&proxy)? != changed_lighting {
            return Err("global lighting write did not read back exactly".into());
        }
        Ok(())
    })();
    let lighting_restore = (|| -> Result<(), Box<dyn Error>> {
        set_lighting(&proxy, original_lighting)?;
        if get_lighting(&proxy)? != original_lighting {
            return Err("global lighting restoration did not read back exactly".into());
        }
        Ok(())
    })();
    if let Err(error) = lighting_restore {
        return Err(format!("global lighting restoration failed: {error}").into());
    }
    lighting_result?;
    println!("SetLighting change, readback, restore, and final readback succeeded.");

    let original_colors = get_colors(&proxy)?;
    let original_esc = esc_color(&original_colors)?;
    // Calibrated vivid device presets.
    let temporary = if original_esc == (255, 84, 0) {
        (0, 255, 159)
    } else {
        (255, 84, 0)
    };
    let single_result = (|| -> Result<(), Box<dyn Error>> {
        set_color(&proxy, temporary)?;
        if esc_color(&get_colors(&proxy)?)? != temporary {
            return Err("single-key color write did not read back".into());
        }
        Ok(())
    })();
    let single_restore = (|| -> Result<(), Box<dyn Error>> {
        set_color(&proxy, original_esc)?;
        if esc_color(&get_colors(&proxy)?)? != original_esc {
            return Err("single-key color restoration did not read back".into());
        }
        Ok(())
    })();
    if let Err(error) = single_restore {
        return Err(format!("single-key color restoration failed: {error}").into());
    }
    single_result?;
    println!("SetKeyColor change, readback, restore, and final readback succeeded.");

    let mut changed_colors = original_colors.clone();
    let (_, red, green, blue) = changed_colors
        .iter_mut()
        .find(|(id, _, _, _)| id == "esc")
        .ok_or("GetKeyColors omitted the esc key")?;
    (*red, *green, *blue) = temporary;
    let bulk_result = (|| -> Result<(), Box<dyn Error>> {
        set_colors(&proxy, &changed_colors)?;
        if get_colors(&proxy)? != changed_colors {
            return Err("bulk color write did not read back exactly".into());
        }
        Ok(())
    })();
    let bulk_restore = (|| -> Result<(), Box<dyn Error>> {
        set_colors(&proxy, &original_colors)?;
        if get_colors(&proxy)? != original_colors {
            return Err("bulk color restoration did not read back exactly".into());
        }
        Ok(())
    })();
    if let Err(error) = bulk_restore {
        return Err(format!("bulk color restoration failed: {error}").into());
    }
    bulk_result?;
    println!("SetKeyColors change, readback, restore, and final readback succeeded.");
    Ok(())
}
