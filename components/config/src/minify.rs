use serde::{Deserialize, Serialize};

use crate::profile::{ProfileFilter, Profiles};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MinifyConfig {
    pub html: Option<MinifyFormat>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MinifyFormat {
    profiles: ProfileFilter,
}

impl Profiles for MinifyFormat {
    fn profiles(&self) -> &ProfileFilter {
        &self.profiles
    }
}
