use crate::muscle::{get_intensity, get_muscle_for_parameter, InteractionType, MuscleState};
use crate::osc::{create_send_socket, send_chatbox_message, SEND_INTERVAL};
use owo_skin::auth::GameAuth;
use owo_skin::client::Client;
use owo_skin::muscles::Muscle;
use owo_skin::sensation::Sensation;
use std::cmp::max;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub fn start_owo_thread(
    contact_states: Arc<Mutex<HashMap<String, MuscleState>>>,
    needs_connect: Arc<Mutex<bool>>,
    muscle_mappings: Arc<Mutex<[(&'static str, Muscle, u8, u8, u8); 10]>>,
    toggle_states: Arc<Mutex<HashMap<String, bool>>>,
    ip_address: Arc<Mutex<Option<String>>>,
) {
    let send_socket = create_send_socket().expect("Failed to create send socket");

    thread::spawn(move || {
        let client = Client::new(GameAuth::default());

        let mut i = 0;
        loop {
            {
                let needs_connect_state = *needs_connect.lock().unwrap();
                if needs_connect_state {
                    println!("Connecting to OWO Application");

                    // Check if we have a specific IP to connect to
                    let ip = ip_address.lock().unwrap().clone();
                    let success = if let Some(ip_str) = ip {
                        println!("Connecting to specific IP: {}", ip_str);
                        // Try to connect to the specific IP
                        client.connect_non_blocking(&(ip_str, 54020))
                    } else {
                        // Use auto-connect if no specific IP is provided
                        client.auto_connect_non_blocking()
                    };

                    if !success {
                        continue;
                    }

                    *needs_connect.lock().unwrap() = false;
                    println!("Connected to OWO Application");
                }

                // Create a list of active muscles
                let mut states = contact_states.lock().unwrap();
                let priority_type = states
                    .iter()
                    .map(|(_, state)| state.interaction_type)
                    .max()
                    .unwrap_or(InteractionType::Touch);

                let mappings = &muscle_mappings.lock().unwrap();
                let active_muscles = states
                    .iter()
                    .filter(|(_, state)| state.interaction_type == priority_type)
                    .filter(|(_, state)| state.should_send_sensation())
                    .filter_map(|(param, state)| get_muscle_for_parameter(param, state, mappings))
                    .collect::<Vec<_>>();

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
                    InteractionType::Touch => Sensation::micro_sensation(
                        100,
                        0.3f32,
                        100,
                        0f32,
                        0f32,
                        0f32,
                        "".to_string(),
                    ),
                    InteractionType::Impact => Sensation::micro_sensation(
                        100,
                        0.2f32,
                        100,
                        0f32,
                        0f32,
                        0f32,
                        "".to_string(),
                    ),
                    InteractionType::Stab => Sensation::micro_sensation(
                        60,
                        0.3f32,
                        100,
                        0f32,
                        0f32,
                        0f32,
                        "".to_string(),
                    ),
                };

                i += 1;
                if !active_muscles.is_empty() {
                    let toggle_states = toggle_states.lock().unwrap();
                    let enabled = toggle_states.get("chatbox").unwrap_or(&false);
                    if *enabled
                        && (i % SEND_INTERVAL == 0 || priority_type != InteractionType::Touch)
                    {
                        let message = format!(
                            "Type: {:#?}\nActive muscles: {}\nIntensity: {}",
                            priority_type,
                            active_muscles.len(),
                            highest_intensity
                        );
                        println!("{}", message);

                        if let Err(e) = send_chatbox_message(&send_socket, &message) {
                            println!("Error sending chatbox message: {}", e);
                        }
                    }

                    client.send_sensation(Sensation::with_muscles(sensation, active_muscles));
                }
            }

            thread::sleep(Duration::from_millis(250));
        }
    });
}
