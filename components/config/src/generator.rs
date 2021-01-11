use once_cell::sync::OnceCell;
use semver::Version;

#[derive(Debug)]
pub struct AppData {
    pub name: String,
    pub bin_name: String,
    pub version: String,
    pub semver: Version,
    pub user_agent: String,
}

pub fn get(generator: Option<AppData>) -> &'static AppData {
    static INSTANCE: OnceCell<AppData> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let generator = generator.unwrap();
        generator
    })
}

pub fn name() -> &'static str {
    &get(None).name
}

pub fn version() -> &'static str {
    &get(None).version
}

pub fn user_agent() -> &'static str {
    &get(None).user_agent
}

pub fn semver() -> &'static Version {
    &get(None).semver
}

pub fn bin_name() -> &'static str {
    &get(None).bin_name
}
