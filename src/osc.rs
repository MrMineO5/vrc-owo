use crate::muscle::{InteractionType, MuscleState};
use vrchat_osc::rosc::{OscMessage, OscPacket, OscType};
use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use vrchat_osc::models::{OscRootNode, OscValue};
use vrchat_osc::VRChatOSC;

pub const PREFIX: &str = "/avatar/parameters/owo_pro/";
pub const SEND_INTERVAL: u64 = 10;

pub async fn setup_osc_listener(
    contact_states: Arc<Mutex<HashMap<String, MuscleState>>>,
    toggle_states: Arc<Mutex<HashMap<String, bool>>>,
) -> Result<Arc<VRChatOSC>, Box<dyn std::error::Error>> {
    let vrchat_osc = VRChatOSC::new(None).await?;

    let root_node = OscRootNode::new().with_avatar();
    let toggle_states_clone = toggle_states.clone();
    vrchat_osc
        .register("owo_pro", root_node, move |packet| {
            if let OscPacket::Message(msg) = packet {
                if let Some(value) = msg.args.get(0) {
                    if !msg.addr.starts_with(PREFIX) {
                        return;
                    }

                    let param = &msg.addr[PREFIX.len()..];

                    if param.starts_with("toggle/") {
                        let (_, toggle_type) = param.split_once('/').unwrap();
                        let mut toggle_states = toggle_states_clone.lock().unwrap();
                        if let OscType::Bool(state) = value {
                            toggle_states.insert(toggle_type.to_string(), *state);
                            println!("Set toggle '{}' to {}", toggle_type, state);
                        }
                        return;
                    }

                    let (muscle, parameter) = param.split_once('/').unwrap();

                    if parameter == "depth" {
                        if let OscType::Float(depth) = value {
                            let mut states = contact_states.lock().unwrap();
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
                            return;
                        }

                        let velocity = parameter[9..].parse::<f32>().unwrap();
                        if let OscType::Bool(state) = value {
                            let mut states = contact_states.lock().unwrap();
                            if let Some(current_state) = states.get_mut(muscle) {
                                if *state {
                                    if current_state.velocity < velocity {
                                        current_state.velocity = velocity;
                                    }
                                } else if current_state.interaction_type != InteractionType::Impact
                                    && current_state.velocity > 0.0
                                {
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
                            return;
                        }

                        match contact_type {
                            "blade" => {
                                let mut states = contact_states.lock().unwrap();
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
        })
        .await?;

    let mut toggle_states = toggle_states.lock().unwrap();
    let toggles: Vec<String> = {
        toggle_states.keys().cloned().collect()
    };
    for toggle in toggles {
        let state = vrchat_osc.get_parameter("/avatar/parameters/owo_pro/", "VRChat-Client-*").await
            .ok()
            .and_then(|state|
                state.first()
                    .and_then(|(_, node)| node.value.as_ref())
                    .and_then(|vals| vals.get(0))
                    .and_then(|v| match v {
                        OscValue::Bool(state) => Some(*state),
                        _ => None,
                    })
            )
            .unwrap_or(false);

        toggle_states.insert(toggle.to_string(), state);
    }

    Ok(vrchat_osc)
}

pub fn create_send_socket() -> std::io::Result<UdpSocket> {
    UdpSocket::bind("0.0.0.0:0")
}

pub fn send_chatbox_message(socket: &UdpSocket, message: &str) -> std::io::Result<()> {
    socket.send_to(
        vrchat_osc::rosc::encoder::encode(&OscPacket::Message(OscMessage {
            addr: "/chatbox/input".to_string(),
            args: (vec![
                OscType::String(message.to_string()),
                OscType::Bool(true),
                OscType::Bool(false),
            ]),
        }))
        .unwrap()
        .as_slice(),
        ("127.0.0.1", 9000),
    )?;
    Ok(())
}