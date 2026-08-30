use std::sync::mpsc::{self, Sender};
use std::thread;

use zbus::blocking::{Connection, Proxy};

const SERVICE: &str = "io.github.AnkoKeyboard";
const PATH: &str = "/io/github/AnkoKeyboard";
const INTERFACE: &str = "io.github.AnkoKeyboard";

pub type Info = (String, u16, u16, u16, u8, String);
pub type Lighting = (u8, u8, u8, u8, u8, bool, u8, u8, u8, u8);
pub type Keymap = Vec<(String, String)>;
pub type MacroEvent = (u16, u8, bool, u8);
pub type MacroDefinition = (u8, Vec<MacroEvent>);
pub type Macros = Vec<MacroDefinition>;

#[derive(Debug)]
pub enum Command {
    GetInfo,
    Refresh,
    GetKeymap(String),
    SetKeymap { bank: String, assignments: Keymap },
    GetLighting,
    SetLighting(Lighting),
    GetMacros,
    SetMacro { id: u8, events: Vec<MacroEvent> },
    DeleteMacro(u8),
    FactoryReset,
}

#[derive(Debug)]
pub enum Event {
    Info(Info),
    Refreshed,
    Keymap { bank: String, assignments: Keymap },
    KeymapApplied(String),
    Lighting(Lighting),
    LightingApplied,
    Macros(Macros),
    MacroApplied,
    FactoryResetApplied,
    Error(String),
}

#[derive(Clone)]
pub struct Client {
    sender: Sender<Command>,
}

impl Client {
    pub fn spawn(events: Sender<Event>) -> Self {
        let (sender, commands) = mpsc::channel();
        thread::Builder::new()
            .name("anko-dbus-client".to_string())
            .spawn(move || {
                while let Ok(command) = commands.recv() {
                    let event = execute(command).unwrap_or_else(Event::Error);
                    if events.send(event).is_err() {
                        break;
                    }
                }
            })
            .expect("failed to spawn D-Bus client worker");
        Self { sender }
    }

    pub fn send(&self, command: Command) {
        let _ = self.sender.send(command);
    }
}

fn execute(command: Command) -> Result<Event, String> {
    let connection = Connection::session().map_err(|error| format!("session bus: {error}"))?;
    let proxy = Proxy::new(&connection, SERVICE, PATH, INTERFACE)
        .map_err(|error| format!("keyboardd proxy: {error}"))?;

    match command {
        Command::GetInfo => proxy
            .call("GetInfo", &())
            .map(Event::Info)
            .map_err(|error| format!("GetInfo: {error}")),
        Command::Refresh => proxy
            .call::<_, _, ()>("Refresh", &())
            .map(|()| Event::Refreshed)
            .map_err(|error| format!("Refresh: {error}")),
        Command::GetKeymap(bank) => proxy
            .call("GetKeymap", &(bank.as_str(),))
            .map(|assignments| Event::Keymap { bank, assignments })
            .map_err(|error| format!("GetKeymap: {error}")),
        Command::SetKeymap { bank, assignments } => proxy
            .call::<_, _, ()>("SetKeymap", &(bank.as_str(), &assignments))
            .map(|()| Event::KeymapApplied(bank))
            .map_err(|error| format!("SetKeymap: {error}")),
        Command::GetLighting => proxy
            .call("GetLighting", &())
            .map(Event::Lighting)
            .map_err(|error| format!("GetLighting: {error}")),
        Command::SetLighting(state) => proxy
            .call::<_, _, ()>("SetLighting", &state)
            .map(|()| Event::LightingApplied)
            .map_err(|error| format!("SetLighting: {error}")),
        Command::GetMacros => proxy
            .call("GetMacros", &())
            .map(Event::Macros)
            .map_err(|error| format!("GetMacros: {error}")),
        Command::SetMacro { id, events } => proxy
            .call::<_, _, ()>("SetMacro", &(id, &events))
            .map(|()| Event::MacroApplied)
            .map_err(|error| format!("SetMacro: {error}")),
        Command::DeleteMacro(id) => proxy
            .call::<_, _, ()>("DeleteMacro", &(id,))
            .map(|()| Event::MacroApplied)
            .map_err(|error| format!("DeleteMacro: {error}")),
        Command::FactoryReset => proxy
            .call::<_, _, ()>("FactoryReset", &())
            .map(|()| Event::FactoryResetApplied)
            .map_err(|error| format!("FactoryReset: {error}")),
    }
}
