use std::path::PathBuf;
use std::sync::Arc;

use collator::Collation;

use compiler::{parser, BuildContext};
use config::{Config, RuntimeOptions, MenuReference};
use locale::Locales;

use crate::Result;

fn compile_file(path: PathBuf) -> Result<String> {
    Ok("".to_string())
}

fn resolve_file(options: &RuntimeOptions, file: &String) -> PathBuf {
    options.source.join(
        utils::url::to_path_separator(file.trim_start_matches("/")))
}

pub fn compile(
    config: &Arc<Config>,
    options: &Arc<RuntimeOptions>,
    locales: &Arc<Locales>,
    collation: &mut Collation) -> Result<()> {
    for (menu, paths) in collation.get_graph().menus.iter() {
        println!("Compiling menu {:#?}", menu);

        match menu {
            MenuReference::File { ref file } => {

                // Create a temporary context
                //let context = BuildContext {
                    //config: Arc::clone(config),
                    //options: Arc::clone(options),
                    //locales: Arc::clone(locales),
                    //collation: Arc::new(collation),
                //};

                //let parser = parser::handlebars(
                    //Arc::new(context),
                    //Arc::clone(locales),
                //)?;

                //file.goo();
                //let res = compile_file(resolve_file(options, file))?;
            }
            _ => todo!()
        }

    }
    Ok(())
}
