use owo_skin::auth::GameAuth;
use owo_skin::client::Client;
use owo_skin::muscles::{Muscle, MuscleWithIntensity};
use owo_skin::sensation::Sensation;
use rosc::{OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};
use slint::Model;
use std::cmp::max;
use std::collections::HashMap;
use std::fs;
use std::net::UdpSocket;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

slint::include_modules!();

const PREFIX: &str = "/avatar/parameters/owo_pro/";
const CONFIG_FILE: &str = "muscle_config.json";

#[derive(Serialize, Deserialize)]
struct MuscleConfig {
    name: String,
    muscle: String,
    intensity_touch: u8,
    intensity_impact: u8,
    intensity_stab: u8,
}

#[derive(Serialize, Deserialize)]
struct Config {
    muscles: Vec<MuscleConfig>,
}

fn load_config() -> Option<Config> {
    let config_path = get_config_path();
    if !config_path.exists() {
        return None;
    }

    match fs::read_to_string(config_path) {
        Ok(contents) => serde_json::from_str(&contents).ok(),
        Err(e) => {
            println!("Error reading config file: {}", e);
            None
        }
    }
}

fn save_config(config: &Config) -> std::io::Result<()> {
    let config_path = get_config_path();
    let json = serde_json::to_string_pretty(config)?;
    fs::write(config_path, json)
}

fn get_config_path() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("vrc-owo");
    fs::create_dir_all(&path).ok();
    path.push(CONFIG_FILE);
    path
}

fn default_muscle_mappings() -> [(&'static str, Muscle, u8, u8, u8); 10] {
    [
        ("Pectoral_R", Muscle::PectoralR, 20, 60, 100),
        ("Pectoral_L", Muscle::PectoralL, 20, 60, 100),
        ("Abdominal_R", Muscle::AbdominalR, 15, 50, 100),
        ("Abdominal_L", Muscle::AbdominalL, 15, 50, 100),
        ("Arm_R", Muscle::ArmR, 15, 30, 80),
        ("Arm_L", Muscle::ArmL, 15, 30, 80),
        ("Dorsal_R", Muscle::DorsalR, 15, 50, 100),
        ("Dorsal_L", Muscle::DorsalL, 15, 50, 100),
        ("Lumbar_R", Muscle::LumbarR, 20, 60, 100),
        ("Lumbar_L", Muscle::LumbarL, 20, 60, 100),
    ]
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
enum InteractionType {
    Touch,
    Impact,
    Stab,
}

#[derive(Clone)]
pub struct MuscleState {
    interaction_type: InteractionType,
    depth: f32,
    velocity: f32,
}

impl MuscleState {
    fn default() -> Self {
        Self {
            interaction_type: InteractionType::Touch,
            depth: 0.0,
            velocity: 0.0,
        }
    }

    fn should_send_sensation(&self) -> bool {
        self.depth > 0.0 || self.velocity > 0.0 && self.interaction_type == InteractionType::Impact
    }
}

const SEND_INTERVAL: u64 = 10;

fn main() -> std::io::Result<()> {
    let muscle_mappings: Arc<Mutex<[(&str, Muscle, u8, u8, u8); 10]>> = Arc::new(Mutex::new(
        if let Some(config) = load_config() {
            let mut mappings = default_muscle_mappings();
            for muscle_config in config.muscles {
                if let Some((_, _, intensity_touch, intensity_impact, intensity_stab)) = 
                    mappings.iter_mut().find(|(name, _, _, _, _)| *name == muscle_config.name) {
                    *intensity_touch = muscle_config.intensity_touch;
                    *intensity_impact = muscle_config.intensity_impact;
                    *intensity_stab = muscle_config.intensity_stab;
                }
            }
            mappings
        } else {
            default_muscle_mappings()
        }
    ));
    
    fn get_muscle_for_parameter(parameter: &str, state: &MuscleState, mappings: &[(&str, Muscle, u8, u8, u8); 10]) -> Option<MuscleWithIntensity> {
        mappings.iter()
                .find(|(param, _, _, _, _)| *param == parameter)
                .map(|(_, muscle, intensity_touch, intensity_impact, intensity_stab)| {
                    let intensity = match state.interaction_type {
                        InteractionType::Touch => *intensity_touch as f32 * state.depth,
                        InteractionType::Impact => *intensity_impact as f32 * state.velocity / 5.0,
                        InteractionType::Stab => *intensity_stab as f32,
                    };
                    MuscleWithIntensity::new(*muscle, intensity as u8)
                })
    }

    fn get_intensity(parameter: &str, state: &MuscleState, mappings: &[(&str, Muscle, u8, u8, u8); 10]) -> Option<u8> {
        mappings.iter()
                .find(|(param, _, _, _, _)| *param == parameter)
                .map(|(_, _, intensity_touch, intensity_impact, intensity_stab)| {
                    let intensity = match state.interaction_type {
                        InteractionType::Touch => *intensity_touch as f32 * state.depth,
                        InteractionType::Impact => *intensity_impact as f32 * state.velocity / 5.0,
                        InteractionType::Stab => *intensity_stab as f32,
                    };
                    intensity as u8
                })
    }
    
    fn get_supported_parameters(mappings: &[(&str, Muscle, u8, u8, u8); 10]) -> Vec<String> {
        mappings
                .iter()
                .map(|(param, _, _, _, _)| param.to_string())
                .collect()
    }

    // Create a UDP socket to listen for OSC messages
    let socket = UdpSocket::bind("127.0.0.1:9001")?;
    let send_socket = UdpSocket::bind("0.0.0.0:0")?;
    println!("Listening for OSC messages on 127.0.0.1:9001");

    // Create a HashMap to store contact states
    let contact_states = Arc::new(Mutex::new(HashMap::new()));
    let needs_connect = Arc::new(Mutex::new(true));
    let toggle_states = Arc::new(Mutex::new(HashMap::<String, bool>::new()));
    
    // Initialize all supported parameters to false
    {
        let mut states = contact_states.lock().unwrap();
        for param in get_supported_parameters(&muscle_mappings.lock().unwrap()) {
            states.insert(param, MuscleState::default());
        }
    }

    let mut buf = [0u8; 1024];

    // Spawn a thread to handle periodic sensation sending
    let contact_states_clone = Arc::clone(&contact_states);
    let needs_connect_clone = Arc::clone(&needs_connect);
    let muscle_mappings_clone = muscle_mappings.clone();
    let toggle_states_clone = Arc::clone(&toggle_states);
    thread::spawn(move || {
        let client = Client::new(GameAuth::default());
        
        let mut i = 0;
        loop {
            {
                let mut needs_connect = needs_connect_clone.lock().unwrap();
                if *needs_connect {
                    println!("Connecting to OWO Application");
                    client.auto_connect();
                    *needs_connect = false;
                    println!("Connected to OWO Application");
                }

                // Create a list of active muscles
                let mut states = contact_states_clone.lock().unwrap();
                let priority_type = states.iter()
                    .map(|(_, state)| state.interaction_type)
                    .max().unwrap_or(InteractionType::Touch);

                let mappings = &muscle_mappings_clone.lock().unwrap();
                let active_muscles: Vec<MuscleWithIntensity> = states
                    .iter()
                    .filter(|(_, state)| state.interaction_type == priority_type)
                    .filter(|(_, state)| state.should_send_sensation())
                    .filter_map(|(param, state)| {
                        get_muscle_for_parameter(param, state, mappings)
                    })
                    .collect();

                let mut highest_intensity = 0;
                states.iter().for_each(|(param, state)| {
                    if let Some(intensity) = get_intensity(param, state, mappings) {
                        highest_intensity = max(highest_intensity, intensity);
                    }
                });
                if priority_type != InteractionType::Touch {
                    // Reset all states to touch
                    states.iter_mut().for_each(|(_, state)| {
                        state.interaction_type = InteractionType::Touch;
                        state.velocity = 0.0;
                    });
                }

                // Only send if there are active muscles
                let sensation = match priority_type {
                    InteractionType::Touch => Sensation::micro_sensation(100, 0.3f32, 100, 0f32, 0f32, 0f32, "".to_string()),
                    InteractionType::Impact => Sensation::micro_sensation(100, 0.2f32, 100, 0f32, 0f32, 0f32, "".to_string()),
                    InteractionType::Stab => Sensation::micro_sensation(60, 0.3f32, 100, 0f32, 0f32, 0f32, "".to_string()),
                };

                i += 1;
                if !active_muscles.is_empty() {
                    let toggle_states = toggle_states_clone.lock().unwrap();
                    let enabled = toggle_states.get("chatbox").unwrap_or(&false);
                    if *enabled && (i % SEND_INTERVAL == 0 || priority_type != InteractionType::Touch) {
                        let message = format!("Type: {:#?}\nActive muscles: {}\nIntensity: {}", priority_type, active_muscles.len(), highest_intensity);
                        println!("{}", message);

                        send_socket.send_to(rosc::encoder::encode(&OscPacket::Message(OscMessage {
                                addr: "/chatbox/input".to_string(),
                                args: (vec![OscType::String(message.to_string()), OscType::Bool(true), OscType::Bool(false)]) 
                        })).unwrap().as_slice(), ("127.0.0.1", 9000)).unwrap();
                    }

                    client.send_sensation(Sensation::with_muscles(sensation, active_muscles));
                } /* else if i % SEND_INTERVAL == 0 {
                    let message = format!("Haptic vest waiting");
                    send_socket.send_to(rosc::encoder::encode(&OscPacket::Message(OscMessage {
                                addr: "/chatbox/input".to_string(),
                                args: (vec![OscType::String(message.to_string()), OscType::Bool(true), OscType::Bool(false)]) 
                        })).unwrap().as_slice(), ("127.0.0.1", 9000)).unwrap();
                } */
            }

            thread::sleep(Duration::from_millis(300));
        }
    });

    // Spawn a thread to handle OSC messages
    let contact_states_clone: Arc<Mutex<HashMap<String, MuscleState>>> = Arc::clone(&contact_states);
    let toggle_states_clone: Arc<Mutex<HashMap<String, bool>>> = Arc::clone(&toggle_states);
    thread::spawn(move || {
        loop {
            match socket.recv_from(&mut buf) {
                Ok((size, _addr)) => {
                    match rosc::decoder::decode_udp(&buf[..size]) {
                        Ok((_, packet)) => {
                            if let OscPacket::Message(msg) = packet {
                                if let Some(value) = msg.args.get(0) {
                                    if !msg.addr.starts_with(PREFIX) {
                                        continue;
                                    }

                                    let param = &msg.addr[PREFIX.len()..];

                                    if param.starts_with("toggle/") {
                                        let (_, toggle_type) = param.split_once('/').unwrap();
                                        let mut toggle_states = toggle_states_clone.lock().unwrap();
                                        if let OscType::Bool(state) = value {
                                            toggle_states.insert(toggle_type.to_string(), *state);
                                        }
                                        continue;
                                    }

                                    let (muscle, parameter) = param.split_once('/').unwrap();

                                    if parameter == "depth" {
                                        if let OscType::Float(depth) = value {
                                            let mut states = contact_states_clone.lock().unwrap();
                                            if let Some(current_state) = states.get_mut(muscle) {
                                                current_state.depth = *depth;
                                            }
                                        } else {
                                            println!("Received non-float value for depth: {}", value);
                                        }
                                    }

                                    if parameter.starts_with("velocity/") {
                                        let toggle_states = toggle_states_clone.lock().unwrap();
                                        let enabled = toggle_states.get("velocity").unwrap_or(&false);
                                        if !*enabled {
                                            continue;
                                        }

                                        let velocity = parameter[9..].parse::<f32>().unwrap();
                                        if let OscType::Bool(state) = value {
                                            let mut states = contact_states_clone.lock().unwrap();
                                            if let Some(current_state) = states.get_mut(muscle) {
                                                if *state {
                                                    if current_state.velocity < velocity {
                                                        current_state.velocity = velocity;
                                                    }
                                                } else if current_state.interaction_type != InteractionType::Impact && current_state.velocity > 0.0 {
                                                    current_state.interaction_type = InteractionType::Impact;
                                                }
                                            }
                                        } else {
                                            println!("Received non-bool value for velocity: {}", value);
                                        }
                                    }

                                    if parameter.starts_with("type/") {
                                        let (_, contact_type) = parameter.split_once('/').unwrap();
                                        let toggle_states = toggle_states_clone.lock().unwrap();
                                        let enabled = toggle_states.get(contact_type).unwrap_or(&false);
                                        if !*enabled {
                                            continue;
                                        }

                                        match contact_type {
                                            "blade" => {
                                                let mut states = contact_states_clone.lock().unwrap();
                                                if let Some(current_state) = states.get_mut(muscle) {
                                                    current_state.interaction_type = InteractionType::Stab;
                                                }
                                            }
                                            _ => {
                                                println!("Received unknown contact type: {}", contact_type);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => println!("Error decoding OSC packet: {}", e),
                    }
                }
                Err(e) => {
                    println!("Error receiving from socket: {}", e);
                    break;
                }
            }
        }
    });

    // Start the UI
    let app = App::new().unwrap();
    {
        let mappings = muscle_mappings.lock().unwrap();
        app.set_muscles(mappings.iter().map(|(name, _, intensity_touch, intensity_impact, intensity_stab)| {
            MuscleData {
                name: name.to_string().into(),
                intensities: MuscleIntensities {
                    touch: *intensity_touch as i32,
                    impact: *intensity_impact as i32,
                    stab: *intensity_stab as i32,
                },
            }
        }).collect::<Vec<MuscleData>>().as_slice().into());
    }
    let app_handle = app.as_weak();
    app.on_update(move || {
        let app = app_handle.unwrap();
        let mut mappings = muscle_mappings.lock().unwrap();
        app.get_muscles().iter().for_each(|muscle| {
            if let Some((_, _, intensity_touch, intensity_impact, intensity_stab)) = mappings.iter_mut().find(|(name, _, _, _, _)| *name == muscle.name.as_str()) {
                *intensity_touch = muscle.intensities.touch as u8;
                *intensity_impact = muscle.intensities.impact as u8;
                *intensity_stab = muscle.intensities.stab as u8;
            }
        });

        // Save config after update
        let config = Config {
            muscles: mappings.iter().map(|(name, muscle, intensity_touch, intensity_impact, intensity_stab)| {
                MuscleConfig {
                    name: name.to_string(),
                    muscle: format!("{:?}", muscle),
                    intensity_touch: *intensity_touch,
                    intensity_impact: *intensity_impact,
                    intensity_stab: *intensity_stab,
                }
            }).collect(),
        };
        if let Err(e) = save_config(&config) {
            println!("Error saving config: {}", e);
        }
    });
    app.on_connect(move || {
        *needs_connect.lock().unwrap() = true;
    });
    app.run().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
