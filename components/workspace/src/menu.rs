use std::collections::HashMap;
use std::path::PathBuf;
use std::borrow::Cow;
use std::sync::{Arc, RwLock};

use once_cell::sync::OnceCell;

use collator::Collate;
use compiler::{parser::Parser, BuildContext, render_markdown};
use config::{Page, CollatedPage, Config, RuntimeOptions, MenuReference};
use locale::Locales;

use crate::Result;

pub fn cache() -> &'static RwLock<HashMap<MenuReference, &'static str>> {
    static INSTANCE: OnceCell<RwLock<HashMap<MenuReference, &'static str>>> = OnceCell::new();
    INSTANCE.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn compile(
    context: &BuildContext,
    parser: &Box<dyn Parser + Send + Sync>) -> Result<()> {

    let config = &context.config;
    let options = &context.options;
    let markdown = options.settings.types.as_ref().unwrap().markdown();

    let collation = context.collation.read().unwrap();
    let lang: &str = collation.get_lang();

    let menus: Vec<(&MenuReference, &Vec<Arc<PathBuf>>)> = collation
        .get_graph()
        .menus
        .iter()
        .collect();

    // Use a cache so we can get reference to the compiled content
    // as &'static lifetimes, see: MenuEntry
    let mut compiled = cache().write().unwrap();

    for (menu, paths) in menus {
        let mut result: String = Default::default();
        match menu {
            MenuReference::File { ref file, ref name } => {
                let file = options.resolve_source(file);
                let page = Page::new(&context.config, &context.options, &file)?;
                let data = CollatedPage { page: &page, lang };
                result = parser.parse(&file, data, true)?;

                if let Some(ext) = file.extension() {
                    let ext = ext.to_string_lossy().into_owned();
                    if markdown.contains(&ext) {
                        result = render_markdown(&mut Cow::from(&result), config);
                    }
                }
            }
            _ => todo!()
        }

        // Use the Box::leak trick to go to &'static strings
        compiled.insert(menu.clone(), Box::leak(Box::new(result)));
    }

    // Now we need to assign the compiled menus to each of the pages
    for (menu, paths) in collation.get_graph().menus.iter() {
        if let Some(compiled_result) = compiled.get(menu) {
            let name = match menu {
                MenuReference::File {ref name, ..} => name, 
                MenuReference::Pages{ref name, ..} => name, 
            };
            for page_path in paths.iter() {
                if let Some(page) = collation.resolve(page_path) {
                    let mut writer = page.write().unwrap();
                    if let Some(menu) = writer.menu.as_mut() {
                        if let Some (ref mut target_menu) = menu.get_mut(name) {
                            target_menu.result = compiled_result;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
