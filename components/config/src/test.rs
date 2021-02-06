use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

static NPX: &str = "npx";
static CYPRESS: &str = "cypress";
static RUN: &str = "run";
static OPTS: &str = "test/cypress.opts";
pub static BASE_URL: &str = "CYPRESS_BASE_URL";

// NOTE: later we may provide hooks for running unit tests
// NOTE: too hence this is `[test.integration]`

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct TestConfig {
    integration: IntegrationTestConfig,
}

impl TestConfig {
    pub fn integration(&self) -> &IntegrationTestConfig {
        &self.integration
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            integration: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(default, rename_all = "kebab-case")]
pub struct IntegrationTestConfig {
    command: String,
    args: Vec<String>,
    env: HashMap<String, String>,
    opts: PathBuf,
}

impl IntegrationTestConfig {
    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn args(&self) -> &Vec<String> {
        &self.args
    }

    pub fn env(&self) -> &HashMap<String, String> {
        &self.env
    }

    pub fn opts(&self) -> &PathBuf {
        &self.opts
    }
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            command: NPX.to_string(),
            args: vec![CYPRESS.to_string(), RUN.to_string()],
            env: HashMap::new(),
            opts: PathBuf::from(OPTS)
        }
    }
}
