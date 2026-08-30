use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use hidapi::HidApi;
use keyboard_core::{
    HardwareSnapshot, Hsv, KeyAction, KeyBank, KeyboardConfig, LightingState, Macro, MacroEvent,
    MacroEventAction, MacroEventKind, Rgb,
};
use keyboard_protocol::{KEY_INDEX_COUNT, KeyboardDevice, PHYSICAL_KEYS, physical_key_by_id};
use zbus::interface;

pub const SERVICE_NAME: &str = "io.github.jkli_2.anko_keyboard_configurator.Daemon";
pub const OBJECT_PATH: &str = "/io/github/jkli_2/anko_keyboard_configurator/Daemon";
pub const INTERFACE_NAME: &str = "io.github.jkli_2.anko_keyboard_configurator.Daemon";

const WORKER_RESPONSE_TIMEOUT: Duration = Duration::from_secs(15);
const PROFILE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(60);

type LightingTuple = (u8, u8, u8, u8, u8, bool, u8, u8, u8, u8);
type ColorAssignment = (String, u8, u8, u8);
type MacroEventTuple = (u16, u8, bool, u8);
type MacroTuple = (u8, Vec<MacroEventTuple>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

impl ConnectionState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Disconnected => "disconnected",
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerSnapshot {
    pub state: ConnectionState,
    pub product_id: u16,
    pub firmware_version: u16,
    pub protocol_version: u16,
    pub layer_count: u8,
    pub last_error: String,
}

impl Default for WorkerSnapshot {
    fn default() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            product_id: 0,
            firmware_version: 0,
            protocol_version: 0,
            layer_count: 0,
            last_error: String::new(),
        }
    }
}

#[derive(Clone)]
pub struct WorkerHandle {
    commands: Sender<Command>,
    snapshot: Arc<Mutex<WorkerSnapshot>>,
}

impl WorkerHandle {
    pub fn spawn() -> Self {
        let (commands, receiver) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(WorkerSnapshot {
            state: ConnectionState::Connecting,
            ..WorkerSnapshot::default()
        }));
        let worker_snapshot = Arc::clone(&snapshot);
        thread::Builder::new()
            .name("anko-hid-worker".into())
            .spawn(move || worker_loop(receiver, worker_snapshot))
            .expect("failed to spawn the HID worker");
        Self { commands, snapshot }
    }

    pub fn snapshot(&self) -> WorkerSnapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn refresh(&self) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::Refresh(reply))
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for HID worker refresh".to_string())?
    }

    pub fn get_keymap(&self, bank: KeyBank) -> Result<Vec<KeyAction>, String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::GetKeymap { bank, reply })
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for HID worker keymap".to_string())?
    }

    pub fn set_key(&self, bank: KeyBank, index: u16, action: KeyAction) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::SetKey {
                bank,
                index,
                action,
                reply,
            })
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for HID worker key write".to_string())?
    }

    pub fn set_keymap(
        &self,
        bank: KeyBank,
        assignments: Vec<(u16, KeyAction)>,
    ) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::SetKeymap {
                bank,
                assignments,
                reply,
            })
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for HID worker keymap write".to_string())?
    }

    pub fn get_lighting(&self) -> Result<LightingState, String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::GetLighting(reply))
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for HID worker lighting state".to_string())?
    }

    pub fn set_lighting(&self, state: LightingState) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::SetLighting { state, reply })
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for HID worker lighting write".to_string())?
    }

    pub fn get_colors(&self) -> Result<Vec<Rgb>, String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::GetColors(reply))
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for HID worker colors".to_string())?
    }

    pub fn set_color(&self, index: u16, color: Rgb) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::SetColor {
                index,
                color,
                reply,
            })
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for HID worker color write".to_string())?
    }

    pub fn set_colors(&self, assignments: Vec<(u16, Rgb)>) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::SetColors { assignments, reply })
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for HID worker color-map write".to_string())?
    }

    pub fn get_macros(&self) -> Result<Vec<Macro>, String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::GetMacros(reply))
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for HID worker macros".to_string())?
    }

    pub fn set_macro(&self, definition: Macro) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::SetMacro { definition, reply })
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for HID worker macro write".to_string())?
    }

    pub fn delete_macro(&self, id: u8) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::DeleteMacro { id, reply })
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for HID worker macro deletion".to_string())?
    }

    pub fn factory_reset(&self) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::FactoryReset(reply))
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(WORKER_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for HID worker factory reset".to_string())?
    }

    pub fn capture_hardware_snapshot(&self) -> Result<HardwareSnapshot, String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::CaptureHardwareSnapshot(reply))
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(PROFILE_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for hardware snapshot".to_string())?
    }

    pub fn restore_hardware_snapshot(&self, snapshot: HardwareSnapshot) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(Command::RestoreHardwareSnapshot { snapshot, reply })
            .map_err(|_| "HID worker has stopped".to_string())?;
        response
            .recv_timeout(PROFILE_RESPONSE_TIMEOUT)
            .map_err(|_| "timed out waiting for hardware snapshot restore".to_string())?
    }
}

enum Command {
    Refresh(Sender<Result<(), String>>),
    GetKeymap {
        bank: KeyBank,
        reply: Sender<Result<Vec<KeyAction>, String>>,
    },
    SetKey {
        bank: KeyBank,
        index: u16,
        action: KeyAction,
        reply: Sender<Result<(), String>>,
    },
    SetKeymap {
        bank: KeyBank,
        assignments: Vec<(u16, KeyAction)>,
        reply: Sender<Result<(), String>>,
    },
    GetLighting(Sender<Result<LightingState, String>>),
    SetLighting {
        state: LightingState,
        reply: Sender<Result<(), String>>,
    },
    GetColors(Sender<Result<Vec<Rgb>, String>>),
    SetColor {
        index: u16,
        color: Rgb,
        reply: Sender<Result<(), String>>,
    },
    SetColors {
        assignments: Vec<(u16, Rgb)>,
        reply: Sender<Result<(), String>>,
    },
    GetMacros(Sender<Result<Vec<Macro>, String>>),
    SetMacro {
        definition: Macro,
        reply: Sender<Result<(), String>>,
    },
    DeleteMacro {
        id: u8,
        reply: Sender<Result<(), String>>,
    },
    CaptureHardwareSnapshot(Sender<Result<HardwareSnapshot, String>>),
    RestoreHardwareSnapshot {
        snapshot: HardwareSnapshot,
        reply: Sender<Result<(), String>>,
    },
    FactoryReset(Sender<Result<(), String>>),
}

struct Worker {
    device: Option<KeyboardDevice>,
    snapshot: Arc<Mutex<WorkerSnapshot>>,
}

impl Worker {
    fn refresh(&mut self) -> Result<(), String> {
        self.device = None;
        self.update_snapshot(WorkerSnapshot {
            state: ConnectionState::Connecting,
            ..WorkerSnapshot::default()
        });
        eprintln!("keyboardd: connecting to 36AE:FDA1 FF00:0002");

        let result = (|| {
            let api = HidApi::new().map_err(|error| error.to_string())?;
            let device = KeyboardDevice::open(&api).map_err(|error| error.to_string())?;
            let config = device.read_config().map_err(|error| error.to_string())?;
            Ok::<_, String>((device, config))
        })();

        match result {
            Ok((device, config)) => {
                eprintln!(
                    "keyboardd: connected, firmware {}, protocol {}",
                    config.firmware_version, config.protocol_version
                );
                self.device = Some(device);
                self.update_snapshot(snapshot_from_config(&config));
                Ok(())
            }
            Err(error) => {
                eprintln!("keyboardd: connection failed: {error}");
                self.update_snapshot(WorkerSnapshot {
                    state: ConnectionState::Error,
                    last_error: error.clone(),
                    ..WorkerSnapshot::default()
                });
                Err(error)
            }
        }
    }

    fn get_keymap(&mut self, bank: KeyBank) -> Result<Vec<KeyAction>, String> {
        eprintln!("keyboardd: reading {bank:?} keymap");
        let result = self
            .device
            .as_ref()
            .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?
            .read_keymap(bank)
            .map_err(|error| error.to_string());
        if let Err(error) = &result {
            eprintln!("keyboardd: keymap read failed: {error}");
            self.device = None;
            self.update_snapshot(WorkerSnapshot {
                state: ConnectionState::Error,
                last_error: error.clone(),
                ..WorkerSnapshot::default()
            });
        }
        result
    }

    fn set_key(&mut self, bank: KeyBank, index: u16, action: KeyAction) -> Result<(), String> {
        eprintln!("keyboardd: writing {bank:?} key index {index}");
        let result = self
            .device
            .as_ref()
            .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?
            .write_key(bank, index, action)
            .map_err(|error| error.to_string());
        self.record_device_error("key write", &result);
        result
    }

    fn set_keymap(
        &mut self,
        bank: KeyBank,
        assignments: &[(u16, KeyAction)],
    ) -> Result<(), String> {
        eprintln!("keyboardd: writing {bank:?} physical keymap");
        let result = (|| {
            let device = self
                .device
                .as_ref()
                .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?;
            let mut complete_map = device
                .read_keymap(bank)
                .map_err(|error| error.to_string())?;
            if complete_map.len() != KEY_INDEX_COUNT {
                return Err(format!(
                    "device returned {} key records; expected {KEY_INDEX_COUNT}",
                    complete_map.len()
                ));
            }
            for &(index, action) in assignments {
                complete_map[index as usize] = action;
            }
            device
                .write_keymap(bank, &complete_map)
                .map_err(|error| error.to_string())
        })();
        self.record_device_error("keymap write", &result);
        result
    }

    fn get_lighting(&mut self) -> Result<LightingState, String> {
        eprintln!("keyboardd: reading global lighting");
        let result = self
            .device
            .as_ref()
            .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?
            .read_lighting()
            .map_err(|error| error.to_string());
        self.record_device_error("lighting read", &result);
        result
    }

    fn set_lighting(&mut self, state: LightingState) -> Result<(), String> {
        eprintln!("keyboardd: writing global lighting effect {}", state.effect);
        let result = self
            .device
            .as_ref()
            .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?
            .write_lighting(state)
            .map_err(|error| error.to_string());
        self.record_device_error("lighting write", &result);
        result
    }

    fn get_colors(&mut self) -> Result<Vec<Rgb>, String> {
        eprintln!("keyboardd: reading per-key colors");
        let result = self
            .device
            .as_ref()
            .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?
            .read_colors()
            .map_err(|error| error.to_string());
        self.record_device_error("color read", &result);
        result
    }

    fn set_color(&mut self, index: u16, color: Rgb) -> Result<(), String> {
        eprintln!("keyboardd: writing color at key index {index}");
        let result = self
            .device
            .as_ref()
            .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?
            .write_color(index, color)
            .map_err(|error| error.to_string());
        self.record_device_error("single-key color write", &result);
        result
    }

    fn set_colors(&mut self, assignments: &[(u16, Rgb)]) -> Result<(), String> {
        eprintln!("keyboardd: writing physical color map");
        let result = (|| {
            let device = self
                .device
                .as_ref()
                .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?;
            let mut complete_map = device.read_colors().map_err(|error| error.to_string())?;
            if complete_map.len() != KEY_INDEX_COUNT {
                return Err(format!(
                    "device returned {} color records; expected {KEY_INDEX_COUNT}",
                    complete_map.len()
                ));
            }
            for &(index, color) in assignments {
                complete_map[index as usize] = color;
            }
            device
                .write_colors(&complete_map)
                .map_err(|error| error.to_string())
        })();
        self.record_device_error("color-map write", &result);
        result
    }

    fn get_macros(&mut self) -> Result<Vec<Macro>, String> {
        eprintln!("keyboardd: reading macro storage");
        let result = self
            .device
            .as_ref()
            .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?
            .read_macros()
            .map_err(|error| error.to_string());
        self.record_device_error("macro read", &result);
        result
    }

    fn set_macro(&mut self, definition: Macro) -> Result<(), String> {
        eprintln!("keyboardd: writing macro M{}", definition.id);
        let result = (|| {
            let device = self
                .device
                .as_ref()
                .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?;
            let mut macros = device.read_macros().map_err(|error| error.to_string())?;
            macros.retain(|current| current.id != definition.id);
            macros.push(definition);
            macros.sort_by_key(|current| current.id);
            device
                .write_macros(&macros)
                .map_err(|error| error.to_string())
        })();
        self.record_device_error("macro write", &result);
        result
    }

    fn delete_macro(&mut self, id: u8) -> Result<(), String> {
        eprintln!("keyboardd: deleting macro M{id}");
        let result = (|| {
            let device = self
                .device
                .as_ref()
                .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?;
            let mut macros = device.read_macros().map_err(|error| error.to_string())?;
            macros.retain(|current| current.id != id);
            device
                .write_macros(&macros)
                .map_err(|error| error.to_string())
        })();
        self.record_device_error("macro deletion", &result);
        result
    }

    fn factory_reset(&mut self) -> Result<(), String> {
        eprintln!("keyboardd: restoring factory values");
        let result = self
            .device
            .as_ref()
            .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?
            .factory_reset()
            .map_err(|error| error.to_string());
        if result.is_ok() {
            self.device = None;
            self.update_snapshot(WorkerSnapshot::default());
        } else {
            self.record_device_error("factory reset", &result);
        }
        result
    }

    fn capture_hardware_snapshot(&mut self) -> Result<HardwareSnapshot, String> {
        eprintln!("keyboardd: capturing complete hardware snapshot");
        let result = self
            .device
            .as_ref()
            .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?
            .capture_hardware_snapshot()
            .map_err(|error| error.to_string());
        self.record_device_error("hardware snapshot", &result);
        result
    }

    fn restore_hardware_snapshot(&mut self, snapshot: &HardwareSnapshot) -> Result<(), String> {
        eprintln!("keyboardd: restoring complete hardware snapshot");
        let result = self
            .device
            .as_ref()
            .ok_or_else(|| "keyboard is not connected; call Refresh".to_string())?
            .restore_hardware_snapshot(snapshot)
            .map_err(|error| error.to_string());
        self.record_device_error("hardware snapshot restore", &result);
        result
    }

    fn record_device_error<T>(&mut self, operation: &str, result: &Result<T, String>) {
        if let Err(error) = result {
            eprintln!("keyboardd: {operation} failed: {error}");
            self.device = None;
            self.update_snapshot(WorkerSnapshot {
                state: ConnectionState::Error,
                last_error: error.clone(),
                ..WorkerSnapshot::default()
            });
        }
    }

    fn update_snapshot(&self, value: WorkerSnapshot) {
        *self
            .snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = value;
    }
}

fn worker_loop(receiver: Receiver<Command>, snapshot: Arc<Mutex<WorkerSnapshot>>) {
    let mut worker = Worker {
        device: None,
        snapshot,
    };
    let _ = worker.refresh();
    while let Ok(command) = receiver.recv() {
        match command {
            Command::Refresh(reply) => {
                let _ = reply.send(worker.refresh());
            }
            Command::GetKeymap { bank, reply } => {
                let _ = reply.send(worker.get_keymap(bank));
            }
            Command::SetKey {
                bank,
                index,
                action,
                reply,
            } => {
                let _ = reply.send(worker.set_key(bank, index, action));
            }
            Command::SetKeymap {
                bank,
                assignments,
                reply,
            } => {
                let _ = reply.send(worker.set_keymap(bank, &assignments));
            }
            Command::GetLighting(reply) => {
                let _ = reply.send(worker.get_lighting());
            }
            Command::SetLighting { state, reply } => {
                let _ = reply.send(worker.set_lighting(state));
            }
            Command::GetColors(reply) => {
                let _ = reply.send(worker.get_colors());
            }
            Command::SetColor {
                index,
                color,
                reply,
            } => {
                let _ = reply.send(worker.set_color(index, color));
            }
            Command::SetColors { assignments, reply } => {
                let _ = reply.send(worker.set_colors(&assignments));
            }
            Command::GetMacros(reply) => {
                let _ = reply.send(worker.get_macros());
            }
            Command::SetMacro { definition, reply } => {
                let _ = reply.send(worker.set_macro(definition));
            }
            Command::DeleteMacro { id, reply } => {
                let _ = reply.send(worker.delete_macro(id));
            }
            Command::CaptureHardwareSnapshot(reply) => {
                let _ = reply.send(worker.capture_hardware_snapshot());
            }
            Command::RestoreHardwareSnapshot { snapshot, reply } => {
                let _ = reply.send(worker.restore_hardware_snapshot(&snapshot));
            }
            Command::FactoryReset(reply) => {
                let _ = reply.send(worker.factory_reset());
            }
        }
    }
    eprintln!("keyboardd: HID worker stopped");
}

fn snapshot_from_config(config: &KeyboardConfig) -> WorkerSnapshot {
    WorkerSnapshot {
        state: ConnectionState::Connected,
        product_id: config.product_id,
        firmware_version: config.firmware_version,
        protocol_version: config.protocol_version,
        layer_count: config.layer_count,
        last_error: String::new(),
    }
}

pub struct KeyboardService {
    worker: WorkerHandle,
}

impl KeyboardService {
    pub fn new(worker: WorkerHandle) -> Self {
        Self { worker }
    }
}

#[interface(name = "io.github.jkli_2.anko_keyboard_configurator.Daemon")]
impl KeyboardService {
    #[zbus(property(emits_changed_signal = "false"))]
    fn connected(&self) -> bool {
        self.worker.snapshot().state == ConnectionState::Connected
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn firmware_version(&self) -> u16 {
        self.worker.snapshot().firmware_version
    }

    #[zbus(property(emits_changed_signal = "false"))]
    fn connection_state(&self) -> String {
        self.worker.snapshot().state.as_str().to_string()
    }

    /// Returns `(state, product_id, firmware, protocol, layer_count, last_error)`.
    fn get_info(&self) -> (String, u16, u16, u16, u8, String) {
        let snapshot = self.worker.snapshot();
        (
            snapshot.state.as_str().to_string(),
            snapshot.product_id,
            snapshot.firmware_version,
            snapshot.protocol_version,
            snapshot.layer_count,
            snapshot.last_error,
        )
    }

    fn refresh(&self) -> zbus::fdo::Result<()> {
        self.worker.refresh().map_err(zbus::fdo::Error::Failed)
    }

    /// Returns physical-key assignments as `(stable_key_id, canonical_action)` pairs.
    fn get_keymap(&self, bank: &str) -> zbus::fdo::Result<Vec<(String, String)>> {
        let bank = parse_bank(bank).map_err(zbus::fdo::Error::InvalidArgs)?;
        let actions = self
            .worker
            .get_keymap(bank)
            .map_err(zbus::fdo::Error::Failed)?;
        Ok(physical_assignments(&actions))
    }

    /// Writes one physical key and returns only after a verified device readback.
    fn set_key(&self, bank: &str, key_id: &str, action: &str) -> zbus::fdo::Result<()> {
        let bank = parse_bank(bank).map_err(zbus::fdo::Error::InvalidArgs)?;
        let key = physical_key_by_id(key_id).ok_or_else(|| {
            zbus::fdo::Error::InvalidArgs(format!("unknown physical key id '{key_id}'"))
        })?;
        let action = parse_action(action).map_err(zbus::fdo::Error::InvalidArgs)?;
        self.worker
            .set_key(bank, key.index, action)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Replaces all 63 physical assignments while preserving unused matrix records.
    fn set_keymap(&self, bank: &str, assignments: Vec<(String, String)>) -> zbus::fdo::Result<()> {
        let bank = parse_bank(bank).map_err(zbus::fdo::Error::InvalidArgs)?;
        let assignments = parse_assignments(assignments).map_err(zbus::fdo::Error::InvalidArgs)?;
        self.worker
            .set_keymap(bank, assignments)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Returns raw device lighting fields without lossy UI-range conversion.
    fn get_lighting(&self) -> zbus::fdo::Result<LightingTuple> {
        self.worker
            .get_lighting()
            .map(lighting_tuple)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Writes global lighting and returns only after exact readback verification.
    #[allow(clippy::too_many_arguments)]
    fn set_lighting(
        &self,
        kind: u8,
        effect: u8,
        brightness: u8,
        speed: u8,
        direction: u8,
        color_enabled: bool,
        single_color_index: u8,
        hue: u8,
        saturation: u8,
        value: u8,
    ) -> zbus::fdo::Result<()> {
        let state = validate_lighting(LightingState {
            kind,
            effect,
            brightness,
            speed,
            direction,
            color_enabled,
            single_color_index,
            hsv: Hsv {
                hue,
                saturation,
                value,
            },
        })
        .map_err(zbus::fdo::Error::InvalidArgs)?;
        self.worker
            .set_lighting(state)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Returns `(stable_key_id, red, green, blue)` for all physical keys.
    fn get_key_colors(&self) -> zbus::fdo::Result<Vec<ColorAssignment>> {
        self.worker
            .get_colors()
            .map(|colors| physical_colors(&colors))
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Writes one physical key color using exact device RGB bytes.
    fn set_key_color(&self, key_id: &str, red: u8, green: u8, blue: u8) -> zbus::fdo::Result<()> {
        let key = physical_key_by_id(key_id).ok_or_else(|| {
            zbus::fdo::Error::InvalidArgs(format!("unknown physical key id '{key_id}'"))
        })?;
        self.worker
            .set_color(
                key.index,
                Rgb {
                    r: red,
                    g: green,
                    b: blue,
                },
            )
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Replaces all physical key colors while preserving unused matrix records.
    fn set_key_colors(&self, assignments: Vec<ColorAssignment>) -> zbus::fdo::Result<()> {
        let assignments =
            parse_color_assignments(assignments).map_err(zbus::fdo::Error::InvalidArgs)?;
        self.worker
            .set_colors(assignments)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Returns understood macro definitions as `(id, events)` tuples. Event fields are
    /// `(delay_ms, kind, pressed, code)`, where kind is mouse/keyboard/vscroll/hscroll.
    fn get_macros(&self) -> zbus::fdo::Result<Vec<MacroTuple>> {
        self.worker
            .get_macros()
            .map(|macros| macros.iter().map(macro_tuple).collect())
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Creates or replaces one of the 16 macro definitions.
    fn set_macro(&self, id: u8, events: Vec<MacroEventTuple>) -> zbus::fdo::Result<()> {
        let definition = parse_macro(id, events).map_err(zbus::fdo::Error::InvalidArgs)?;
        self.worker
            .set_macro(definition)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Deletes one macro definition while preserving all other slots.
    fn delete_macro(&self, id: u8) -> zbus::fdo::Result<()> {
        validate_macro_id(id).map_err(zbus::fdo::Error::InvalidArgs)?;
        self.worker
            .delete_macro(id)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Returns a versioned JSON snapshot containing every persistent hardware region.
    fn capture_hardware_snapshot(&self) -> zbus::fdo::Result<String> {
        let snapshot = self
            .worker
            .capture_hardware_snapshot()
            .map_err(zbus::fdo::Error::Failed)?;
        serde_json::to_string(&snapshot)
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }

    /// Atomically restores a complete JSON hardware snapshot with verified rollback.
    fn restore_hardware_snapshot(&self, snapshot_json: &str) -> zbus::fdo::Result<()> {
        let snapshot: HardwareSnapshot = serde_json::from_str(snapshot_json)
            .map_err(|error| zbus::fdo::Error::InvalidArgs(error.to_string()))?;
        self.worker
            .restore_hardware_snapshot(snapshot)
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Destructively restores the keyboard's onboard factory values.
    fn factory_reset(&self) -> zbus::fdo::Result<()> {
        self.worker
            .factory_reset()
            .map_err(zbus::fdo::Error::Failed)
    }
}

pub fn run_service() -> zbus::Result<()> {
    let service = KeyboardService::new(WorkerHandle::spawn());
    let _connection = zbus::blocking::connection::Builder::session()?
        .name(SERVICE_NAME)?
        .serve_at(OBJECT_PATH, service)?
        .build()?;
    eprintln!("keyboardd: serving {INTERFACE_NAME} at {OBJECT_PATH}");
    loop {
        thread::park();
    }
}

fn parse_bank(bank: &str) -> Result<KeyBank, String> {
    match bank {
        "base" => Ok(KeyBank::Base),
        "fn" => Ok(KeyBank::Fn),
        _ => Err("bank must be 'base' or 'fn'".to_string()),
    }
}

fn physical_assignments(actions: &[KeyAction]) -> Vec<(String, String)> {
    PHYSICAL_KEYS
        .iter()
        .filter_map(|key| {
            actions
                .get(key.index as usize)
                .map(|action| (key.id.to_string(), canonical_action(*action)))
        })
        .collect()
}

fn lighting_tuple(state: LightingState) -> LightingTuple {
    (
        state.kind,
        state.effect,
        state.brightness,
        state.speed,
        state.direction,
        state.color_enabled,
        state.single_color_index,
        state.hsv.hue,
        state.hsv.saturation,
        state.hsv.value,
    )
}

fn macro_tuple(definition: &Macro) -> MacroTuple {
    (
        definition.id,
        definition.events.iter().map(macro_event_tuple).collect(),
    )
}

fn macro_event_tuple(event: &MacroEvent) -> MacroEventTuple {
    let kind = match event.kind {
        MacroEventKind::MouseButton => 0,
        MacroEventKind::Keyboard => 1,
        MacroEventKind::VerticalScroll => 2,
        MacroEventKind::HorizontalScroll => 3,
    };
    (
        event.delay_ms,
        kind,
        event.action == MacroEventAction::Press,
        event.code,
    )
}

fn validate_macro_id(id: u8) -> Result<(), String> {
    if id <= 15 {
        Ok(())
    } else {
        Err("macro id must be in 0..=15".to_string())
    }
}

fn parse_macro(id: u8, events: Vec<MacroEventTuple>) -> Result<Macro, String> {
    validate_macro_id(id)?;
    if events.is_empty() {
        return Err("a saved macro must contain at least one event".to_string());
    }
    let events = events
        .into_iter()
        .map(|(delay_ms, kind, pressed, code)| {
            if delay_ms > 10_000 {
                return Err("macro delay must be in 0..=10000 ms".to_string());
            }
            let kind = match kind {
                0 => MacroEventKind::MouseButton,
                1 => MacroEventKind::Keyboard,
                2 => MacroEventKind::VerticalScroll,
                3 => MacroEventKind::HorizontalScroll,
                _ => return Err("macro event kind must be in 0..=3".to_string()),
            };
            Ok(MacroEvent {
                delay_ms,
                kind,
                action: if pressed {
                    MacroEventAction::Press
                } else {
                    MacroEventAction::Release
                },
                code,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Macro { id, events })
}

fn validate_lighting(state: LightingState) -> Result<LightingState, String> {
    if state.kind != 1 {
        return Err("lighting kind must be 1 for this keyboard".to_string());
    }
    if state.effect > 19 {
        return Err("lighting effect must be in 0..=19".to_string());
    }
    if state.effect == 0 && state.color_enabled {
        return Err("color_enabled must be false when lighting effect is off".to_string());
    }
    Ok(state)
}

fn physical_colors(colors: &[Rgb]) -> Vec<ColorAssignment> {
    PHYSICAL_KEYS
        .iter()
        .filter_map(|key| {
            colors
                .get(key.index as usize)
                .map(|color| (key.id.to_string(), color.r, color.g, color.b))
        })
        .collect()
}

fn parse_color_assignments(assignments: Vec<ColorAssignment>) -> Result<Vec<(u16, Rgb)>, String> {
    if assignments.len() != PHYSICAL_KEYS.len() {
        return Err(format!(
            "color map must contain exactly {} physical assignments; got {}",
            PHYSICAL_KEYS.len(),
            assignments.len()
        ));
    }

    let mut seen = HashSet::with_capacity(assignments.len());
    let mut parsed = Vec::with_capacity(assignments.len());
    for (key_id, red, green, blue) in assignments {
        let key = physical_key_by_id(&key_id)
            .ok_or_else(|| format!("unknown physical key id '{key_id}'"))?;
        if !seen.insert(key.id) {
            return Err(format!("duplicate physical key id '{key_id}'"));
        }
        parsed.push((
            key.index,
            Rgb {
                r: red,
                g: green,
                b: blue,
            },
        ));
    }
    Ok(parsed)
}

fn canonical_action(action: KeyAction) -> String {
    match action {
        KeyAction::Keyboard { modifiers, usage } => {
            format!("keyboard/{usage}/{modifiers}")
        }
        KeyAction::FunctionLayer => "function-layer".to_string(),
        KeyAction::Consumer { usage } => format!("consumer/{usage}"),
        KeyAction::MouseButton {
            buttons,
            vertical_wheel,
        } => format!("mouse-button/{buttons}/{vertical_wheel}"),
        KeyAction::MouseMove { x, y, wheel } => format!("mouse-move/{x}/{y}/{wheel}"),
        KeyAction::Macro { id } => format!("macro/{id}"),
        KeyAction::Firmware { code } => format!("firmware/{code}"),
        KeyAction::Power { code } => format!("power/{code}"),
        KeyAction::Raw { kind, codes } => {
            format!("raw/{kind}/{}/{}/{}", codes[0], codes[1], codes[2])
        }
    }
}

fn parse_action(value: &str) -> Result<KeyAction, String> {
    let parts: Vec<_> = value.split('/').collect();
    let invalid = || format!("invalid canonical action '{value}'");
    let u8_at = |index: usize| {
        parts
            .get(index)
            .ok_or_else(invalid)?
            .parse::<u8>()
            .map_err(|_| invalid())
    };
    let i8_at = |index: usize| {
        parts
            .get(index)
            .ok_or_else(invalid)?
            .parse::<i8>()
            .map_err(|_| invalid())
    };
    let u16_at = |index: usize| {
        parts
            .get(index)
            .ok_or_else(invalid)?
            .parse::<u16>()
            .map_err(|_| invalid())
    };

    match parts.as_slice() {
        ["function-layer"] => Ok(KeyAction::FunctionLayer),
        ["keyboard", _, _] => Ok(KeyAction::Keyboard {
            usage: u8_at(1)?,
            modifiers: u8_at(2)?,
        }),
        ["consumer", _] => Ok(KeyAction::Consumer { usage: u16_at(1)? }),
        ["mouse-button", _, _] => Ok(KeyAction::MouseButton {
            buttons: u8_at(1)?,
            vertical_wheel: i8_at(2)?,
        }),
        ["mouse-move", _, _, _] => Ok(KeyAction::MouseMove {
            x: i8_at(1)?,
            y: i8_at(2)?,
            wheel: i8_at(3)?,
        }),
        ["macro", _] => {
            let id = u8_at(1)?;
            if id > 15 {
                return Err("macro id must be in 0..=15".to_string());
            }
            Ok(KeyAction::Macro { id })
        }
        ["firmware", _] => Ok(KeyAction::Firmware { code: u8_at(1)? }),
        ["power", _] => Ok(KeyAction::Power { code: u8_at(1)? }),
        ["raw", _, _, _, _] => Ok(KeyAction::Raw {
            kind: u8_at(1)?,
            codes: [u8_at(2)?, u8_at(3)?, u8_at(4)?],
        }),
        _ => Err(invalid()),
    }
}

fn parse_assignments(assignments: Vec<(String, String)>) -> Result<Vec<(u16, KeyAction)>, String> {
    if assignments.len() != PHYSICAL_KEYS.len() {
        return Err(format!(
            "keymap must contain exactly {} physical assignments; got {}",
            PHYSICAL_KEYS.len(),
            assignments.len()
        ));
    }

    let mut seen = HashSet::with_capacity(assignments.len());
    let mut parsed = Vec::with_capacity(assignments.len());
    for (key_id, action) in assignments {
        let key = physical_key_by_id(&key_id)
            .ok_or_else(|| format!("unknown physical key id '{key_id}'"))?;
        if !seen.insert(key.id) {
            return Err(format!("duplicate physical key id '{key_id}'"));
        }
        parsed.push((key.index, parse_action(&action)?));
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_bank_names_are_strict() {
        assert_eq!(parse_bank("base"), Ok(KeyBank::Base));
        assert_eq!(parse_bank("fn"), Ok(KeyBank::Fn));
        assert_eq!(parse_bank("1"), Err("bank must be 'base' or 'fn'".into()));
    }

    #[test]
    fn macro_tuples_are_strict_and_round_trip() {
        let definition = parse_macro(15, vec![(25, 1, true, 4), (10, 1, false, 4)]).unwrap();
        assert_eq!(
            macro_tuple(&definition),
            (15, vec![(25, 1, true, 4), (10, 1, false, 4)])
        );

        assert!(parse_macro(16, vec![(0, 1, true, 4)]).is_err());
        assert!(parse_macro(0, Vec::new()).is_err());
        assert!(parse_macro(0, vec![(10_001, 1, true, 4)]).is_err());
        assert!(parse_macro(0, vec![(0, 4, true, 4)]).is_err());
    }

    #[test]
    fn actions_have_stable_semantic_strings() {
        let cases = [
            (
                KeyAction::Keyboard {
                    modifiers: 1,
                    usage: 4,
                },
                "keyboard/4/1",
            ),
            (KeyAction::FunctionLayer, "function-layer"),
            (KeyAction::Consumer { usage: 226 }, "consumer/226"),
            (
                KeyAction::MouseButton {
                    buttons: 3,
                    vertical_wheel: -1,
                },
                "mouse-button/3/-1",
            ),
            (
                KeyAction::MouseMove {
                    x: -2,
                    y: 4,
                    wheel: 1,
                },
                "mouse-move/-2/4/1",
            ),
            (KeyAction::Macro { id: 15 }, "macro/15"),
            (KeyAction::Firmware { code: 3 }, "firmware/3"),
            (KeyAction::Power { code: 2 }, "power/2"),
            (
                KeyAction::Raw {
                    kind: 19,
                    codes: [0, 0, 0],
                },
                "raw/19/0/0/0",
            ),
        ];
        for (action, expected) in cases {
            assert_eq!(canonical_action(action), expected);
            assert_eq!(parse_action(expected), Ok(action));
        }
    }

    #[test]
    fn malformed_actions_are_rejected() {
        for value in [
            "",
            "keyboard/4",
            "keyboard/256/0",
            "consumer/-1",
            "mouse-move/1/2",
            "macro/16",
            "firmware/1/2",
            "raw/1/2/3",
        ] {
            assert!(parse_action(value).is_err(), "accepted {value}");
        }
    }

    #[test]
    fn full_keymap_validation_rejects_incomplete_duplicate_and_unknown_ids() {
        let valid: Vec<_> = PHYSICAL_KEYS
            .iter()
            .map(|key| (key.id.to_string(), "keyboard/0/0".to_string()))
            .collect();
        assert_eq!(parse_assignments(valid.clone()).unwrap().len(), 63);

        assert!(parse_assignments(valid[..62].to_vec()).is_err());

        let mut duplicate = valid.clone();
        duplicate[62].0 = duplicate[0].0.clone();
        assert!(parse_assignments(duplicate).is_err());

        let mut unknown = valid;
        unknown[0].0 = "not-a-key".to_string();
        assert!(parse_assignments(unknown).is_err());
    }

    #[test]
    fn dbus_keymap_contains_only_physical_keys() {
        let actions = vec![
            KeyAction::Keyboard {
                modifiers: 0,
                usage: 0,
            };
            75
        ];
        let assignments = physical_assignments(&actions);
        assert_eq!(assignments.len(), 63);
        assert_eq!(assignments.first().map(|item| item.0.as_str()), Some("esc"));
        assert_eq!(assignments.last().map(|item| item.0.as_str()), Some("fn"));
        assert!(!assignments.iter().any(|item| item.0 == "<unused>"));
    }

    #[test]
    fn lighting_validation_matches_known_device_limits() {
        let state = LightingState {
            kind: 1,
            effect: 19,
            brightness: 4,
            speed: 2,
            direction: 1,
            color_enabled: true,
            single_color_index: 0,
            hsv: Hsv {
                hue: 219,
                saturation: 255,
                value: 255,
            },
        };
        assert_eq!(validate_lighting(state), Ok(state));
        assert_eq!(
            lighting_tuple(state),
            (1, 19, 4, 2, 1, true, 0, 219, 255, 255)
        );

        assert!(validate_lighting(LightingState { kind: 2, ..state }).is_err());
        assert!(
            validate_lighting(LightingState {
                effect: 20,
                ..state
            })
            .is_err()
        );
        assert!(
            validate_lighting(LightingState {
                effect: 0,
                color_enabled: true,
                ..state
            })
            .is_err()
        );
    }

    #[test]
    fn color_maps_use_only_physical_keys_and_require_complete_unique_ids() {
        let colors = vec![Rgb { r: 1, g: 2, b: 3 }; KEY_INDEX_COUNT];
        let public = physical_colors(&colors);
        assert_eq!(public.len(), 63);
        assert_eq!(public[0], ("esc".to_string(), 1, 2, 3));

        assert_eq!(parse_color_assignments(public.clone()).unwrap().len(), 63);
        assert!(parse_color_assignments(public[..62].to_vec()).is_err());

        let mut duplicate = public.clone();
        duplicate[62].0 = duplicate[0].0.clone();
        assert!(parse_color_assignments(duplicate).is_err());

        let mut unknown = public;
        unknown[0].0 = "not-a-key".to_string();
        assert!(parse_color_assignments(unknown).is_err());
    }
}
