use once_cell::sync::OnceCell;

#[derive(Debug, Default)]
pub struct AppData {
    pub name: String,
    pub version: String,
    pub id: String,
}

pub fn get(generator: Option<AppData>) -> &'static AppData {
    static INSTANCE: OnceCell<AppData> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut generator = generator.unwrap();
        generator.id = format!("{}/{}", generator.name, generator.version);
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
