use owo_skin::muscles::{Muscle, MuscleWithIntensity};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum InteractionType {
    Touch,
    Impact,
    Stab,
}

#[derive(Clone)]
pub struct MuscleState {
    pub interaction_type: InteractionType,
    pub depth: f32,
    pub velocity: f32,
}

impl MuscleState {
    pub fn default() -> Self {
        Self {
            interaction_type: InteractionType::Touch,
            depth: 0.0,
            velocity: 0.0,
        }
    }

    pub fn should_send_sensation(&self) -> bool {
        self.depth > 0.0 || self.velocity > 0.0 && self.interaction_type == InteractionType::Impact
    }
}

pub fn default_muscle_mappings() -> [(&'static str, Muscle, u8, u8, u8); 10] {
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

pub fn get_muscle_for_parameter(
    parameter: &str,
    state: &MuscleState,
    mappings: &[(&str, Muscle, u8, u8, u8); 10],
) -> Option<MuscleWithIntensity> {
    mappings
        .iter()
        .find(|(param, _, _, _, _)| *param == parameter)
        .map(
            |(_, muscle, intensity_touch, intensity_impact, intensity_stab)| {
                let intensity = match state.interaction_type {
                    InteractionType::Touch => *intensity_touch as f32 * state.depth,
                    InteractionType::Impact => *intensity_impact as f32 * state.velocity / 5.0,
                    InteractionType::Stab => *intensity_stab as f32,
                };
                MuscleWithIntensity::new(*muscle, intensity as u8)
            },
        )
}

pub fn get_intensity(
    parameter: &str,
    state: &MuscleState,
    mappings: &[(&str, Muscle, u8, u8, u8); 10],
) -> Option<u8> {
    mappings
        .iter()
        .find(|(param, _, _, _, _)| *param == parameter)
        .map(
            |(_, _, intensity_touch, intensity_impact, intensity_stab)| {
                let intensity = match state.interaction_type {
                    InteractionType::Touch => *intensity_touch as f32 * state.depth,
                    InteractionType::Impact => *intensity_impact as f32 * state.velocity / 5.0,
                    InteractionType::Stab => *intensity_stab as f32,
                };
                intensity as u8
            },
        )
}

pub fn get_supported_parameters(mappings: &[(&str, Muscle, u8, u8, u8); 10]) -> Vec<String> {
    mappings
        .iter()
        .map(|(param, _, _, _, _)| param.to_string())
        .collect()
}