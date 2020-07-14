use locale::Locales;

#[derive(Debug)]
pub struct Context {
    pub locales: Locales,
}

impl Context {
    pub fn new(locales: Locales) -> Self {
        Self { locales }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self { locales: Default::default() }
    }
}
