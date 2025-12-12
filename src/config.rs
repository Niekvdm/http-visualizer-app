use std::env;

pub struct Config {
    pub port: u16,
    pub frontend_path: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            frontend_path: env::var("FRONTEND_PATH").ok(),
        }
    }
}
