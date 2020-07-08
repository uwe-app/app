use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum BuildTag {
    Custom(String),
    Debug,
    Release,
}

impl Default for BuildTag {
    fn default() -> Self {
        BuildTag::Debug
    }
}

impl BuildTag {
    pub fn get_path_name(&self) -> String {
        match self {
            BuildTag::Debug => return "debug".to_string(),
            BuildTag::Release => return "release".to_string(),
            BuildTag::Custom(s) => return s.to_string(),
        }
    }

    pub fn get_node_env(&self, debug: Option<String>, release: Option<String>) -> String {
        match self {
            BuildTag::Debug => {
                if let Some(env) = debug {
                    return env;
                }
                return "development".to_string();
            }
            BuildTag::Release => {
                if let Some(env) = release {
                    return env;
                }
                return "production".to_string();
            }
            BuildTag::Custom(s) => return s.to_string(),
        }
    }
}

