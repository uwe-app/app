use std::env;

use crate::{Error, Result};

const LOG_ENV_NAME: &'static str = "UWE_LOG";

pub fn log_level(level: &str) -> Result<()> {
    match level {
        "trace" => env::set_var(LOG_ENV_NAME, level),
        "debug" => env::set_var(LOG_ENV_NAME, level),
        "info" => env::set_var(LOG_ENV_NAME, level),
        "warn" => env::set_var(LOG_ENV_NAME, level),
        "error" => env::set_var(LOG_ENV_NAME, level),
        _ => {
            // Jump a few hoops to pretty print this message
            env::set_var(LOG_ENV_NAME, "error");
            pretty_env_logger::init_custom_env(LOG_ENV_NAME);
            return Err(Error::UnknownLogLevel(level.to_string()));
        }
    }

    pretty_env_logger::init_custom_env(LOG_ENV_NAME);

    Ok(())
}
