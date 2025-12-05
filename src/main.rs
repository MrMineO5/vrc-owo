use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use owo_skin::muscles::Muscle;
use vrc_owo::config::load_config;
use vrc_owo::muscle::{default_muscle_mappings, get_supported_parameters, MuscleState};
use vrc_owo::osc::setup_osc_listener;
use vrc_owo::owo_thread::start_owo_thread;
use vrc_owo::ui::setup_ui;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize muscle mappings from config or defaults
    let muscle_mappings: Arc<Mutex<[(&str, Muscle, u8, u8, u8); 10]>> =
        Arc::new(Mutex::new(if let Some(config) = load_config() {
            let mut mappings = default_muscle_mappings();
            for muscle_config in config.muscles {
                if let Some((_, _, intensity_touch, intensity_impact, intensity_stab)) = mappings
                    .iter_mut()
                    .find(|(name, _, _, _, _)| *name == muscle_config.name)
                {
                    *intensity_touch = muscle_config.intensity_touch;
                    *intensity_impact = muscle_config.intensity_impact;
                    *intensity_stab = muscle_config.intensity_stab;
                }
            }
            mappings
        } else {
            default_muscle_mappings()
        }));

    // Create shared state
    let contact_states = Arc::new(Mutex::new(HashMap::new()));
    let needs_connect = Arc::new(Mutex::new(true));
    let toggle_states = Arc::new(Mutex::new(HashMap::<String, bool>::new()));
    let ip_address = Arc::new(Mutex::new(None::<String>));

    // Initialize all supported parameters
    {
        let mut states = contact_states.lock().unwrap();
        for param in get_supported_parameters(&muscle_mappings.lock().unwrap()) {
            states.insert(param, MuscleState::default());
        }
    }

    // Load IP address from config if available
    if let Some(config) = load_config() {
        if let Some(ip) = config.ip_address {
            if !ip.is_empty() {
                let mut ip_lock = ip_address.lock().unwrap();
                *ip_lock = Some(ip);
            }
        }
    }

    // Start the OWO thread
    start_owo_thread(
        contact_states.clone(),
        needs_connect.clone(),
        muscle_mappings.clone(),
        toggle_states.clone(),
        ip_address.clone(),
    );

    // Setup OSC listener
    let _vrcchat_osc = setup_osc_listener(contact_states.clone(), toggle_states.clone())
        .await
        .unwrap();

    // Start the UI
    setup_ui(muscle_mappings.clone(), needs_connect.clone(), ip_address.clone())
}
