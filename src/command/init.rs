use std::path::PathBuf;

use crate::utils;
use crate::Error;

use log::info;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "template"]
struct Asset;

#[derive(Debug)]
pub struct InitOptions {
    pub target: PathBuf,
}

pub fn init(options: InitOptions) -> Result<(), Error> {

    let template_name = "newcss";

    info!("init {} using {}", options.target.display(), template_name);

    let files: Vec<&str> = vec![
        "site/index.md",
        "site/data.toml",
        "site/layout.hbs",
        "site/template/header.hbs",
        "site/template/footer.hbs",
    ];

    for f in files.iter() {
        let mut o = options.target.clone();
        let mut s = template_name.clone().to_string();
        s.push('/');
        s.push_str(f);
        o.push(f);
        info!("init {} -> {}", s, o.display());
        let dir = Asset::get(&s);
        match dir {
            Some(f) => {
                utils::write_all(o, &f)?;
            },
            None  => return Err(Error::new("template source file not found".to_string()))
        }
    }

    Ok(())
}
