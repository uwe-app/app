use once_cell::sync::OnceCell;

pub fn version(version: Option<String>) -> &'static String {
    static INSTANCE: OnceCell<String> = OnceCell::new();
    INSTANCE.get_or_init(|| version.as_ref().unwrap().to_string())
}
