use semver::Version;
use once_cell::sync::OnceCell;

#[derive(Debug)]
pub struct AppData {
    pub name: String,
    pub version: String,
    pub semver: Version,
    pub id: String,
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

pub fn id() -> &'static str {
    &get(None).id
}

pub fn semver() -> &'static Version {
    &get(None).semver
}
