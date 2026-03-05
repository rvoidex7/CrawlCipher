use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Debug, Default)]
pub struct AppConfig {
    pub expedition: ExpeditionConfig,
}

#[derive(Deserialize, Debug, Default)]
pub struct ExpeditionConfig {
    pub spawn: SpawnConfig,
    pub boss_waves: Vec<BossWaveConfig>,
    #[allow(dead_code)]
    pub scoring: ScoringConfig,
}

#[derive(Deserialize, Debug, Default)]
pub struct SpawnConfig {
    pub initial_snake_length: i32,
    #[allow(dead_code)]
    pub initial_energy: i32,
    pub initial_bot_count: i32,
}

#[derive(Deserialize, Debug, Default)]
pub struct BossWaveConfig {
    pub wave_number: i32,
    pub trigger_time_seconds: i32,
    pub boss_type: String,
    pub multiplier: f32,
    #[allow(dead_code)]
    pub additional_bots: i32,
    #[allow(dead_code)]
    pub bot_speed_multiplier: f32,
}

#[derive(Deserialize, Debug, Default)]
pub struct ScoringConfig {
    #[allow(dead_code)]
    pub food: i32,
    #[allow(dead_code)]
    pub snail: i32,
    #[allow(dead_code)]
    pub bot_kill_base: i32,
    #[allow(dead_code)]
    pub boss_kill: i32,
    #[allow(dead_code)]
    pub survival_per_second: i32,
}

pub fn load_config(path: &str) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: AppConfig = serde_json::from_str(&content)?;
    Ok(config)
}
