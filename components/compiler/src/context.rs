use locale::Locales;

#[derive(Debug)]
pub struct Context {
    pub livereload: Option<String>,
    pub locales: Locales,
}

impl Context {
    pub fn new(locales: Locales) -> Self {
        Self {
            locales,
            livereload: None,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self {
            locales: Default::default(),
            livereload: None,
        }
    }
}
