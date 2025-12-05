use crate::config::{Config, MuscleConfig, save_config};
use slint::Model;
use std::sync::{Arc, Mutex};
use owo_skin::muscles::Muscle;

slint::include_modules!();

pub fn setup_ui(
    muscle_mappings: Arc<Mutex<[(&'static str, Muscle, u8, u8, u8); 10]>>,
    needs_connect: Arc<Mutex<bool>>,
    ip_address: Arc<Mutex<Option<String>>>,
) -> Result<(), std::io::Error> {
    let app = App::new().unwrap();

    // Load IP address from config if available
    if let Some(config) = crate::config::load_config() {
        if let Some(ip) = config.ip_address {
            app.set_ip_address(ip.into());
        }
    }

    {
        let mappings = muscle_mappings.lock().unwrap();
        app.set_muscles(
            mappings
                .iter()
                .map(
                    |(name, _, intensity_touch, intensity_impact, intensity_stab)| MuscleData {
                        name: name.to_string().into(),
                        intensities: MuscleIntensities {
                            touch: *intensity_touch as i32,
                            impact: *intensity_impact as i32,
                            stab: *intensity_stab as i32,
                        },
                    },
                )
                .collect::<Vec<MuscleData>>()
                .as_slice()
                .into(),
        );
    }
    let app_handle = app.as_weak();
    app.on_update(move || {
        let app = app_handle.unwrap();
        let mut mappings = muscle_mappings.lock().unwrap();
        app.get_muscles().iter().for_each(|muscle| {
            if let Some((_, _, intensity_touch, intensity_impact, intensity_stab)) = mappings
                .iter_mut()
                .find(|(name, _, _, _, _)| *name == muscle.name.as_str())
            {
                *intensity_touch = muscle.intensities.touch as u8;
                *intensity_impact = muscle.intensities.impact as u8;
                *intensity_stab = muscle.intensities.stab as u8;
            }
        });

        // Save config after update
        let config = Config {
            muscles: mappings
                .iter()
                .map(
                    |(name, muscle, intensity_touch, intensity_impact, intensity_stab)| {
                        MuscleConfig {
                            name: name.to_string(),
                            muscle: format!("{:?}", muscle),
                            intensity_touch: *intensity_touch,
                            intensity_impact: *intensity_impact,
                            intensity_stab: *intensity_stab,
                        }
                    },
                )
                .collect(),
            ip_address: Some(app.get_ip_address().to_string()),
        };
        if let Err(e) = save_config(&config) {
            println!("Error saving config: {}", e);
        }
    });

    let ip_address_clone = ip_address.clone();
    let needs_connect_clone = needs_connect.clone();
    app.on_connect(move || {
        let mut ip_addr = ip_address_clone.lock().unwrap();
        *ip_addr = None;

        *needs_connect_clone.lock().unwrap() = true;
    });

    app.on_connect_ip(move |ip| {
        let mut ip_addr = ip_address.lock().unwrap();
        *ip_addr = Some(ip.to_string());

        let mut needs_connect = needs_connect.lock().unwrap();
        *needs_connect = true;
    });

    app.run()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
