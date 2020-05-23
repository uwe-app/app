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
    pub template: String,
}

static TEMPLATES: [&str; 2] = ["newcss", "tacit"];

fn copy_file(f: &str, template_name: &str, output: &mut PathBuf) -> Result<(), Error> {
    let mut s = template_name.clone().to_string();
    s.push('/');
    s.push_str(f);
    output.push(f);
    info!("init {} -> {}", s, output.display());
    let dir = Asset::get(&s);
    match dir {
        Some(f) => {
            utils::write_all(output, &f)?;
        },
        None  => return Err(
            Error::new("template source file not found".to_string()))
    }
    Ok(())
}

pub fn init(options: InitOptions) -> Result<(), Error> {

    let template_name = options.template;

    if !TEMPLATES.contains(&template_name.as_str()) {
        return Err(Error::new(format!("unknown template {}", &template_name)))
    }

    let common_name = "common";

    info!("init {} using {}", options.target.display(), template_name);

    let common_files: Vec<&str> = vec![
        "site/.gitignore",
        "site/index.md",
        "site/data.toml",
        "site/layout.hbs",
        "site/template/header.hbs",
        "site/template/footer.hbs",
    ];

    let template_files = vec![
        "site/assets/style.css"
    ];

    for f in common_files.iter() {
        let mut o = options.target.clone();
        copy_file(f, common_name, &mut o)?;
    }

    for f in template_files.iter() {
        let mut o = options.target.clone();
        copy_file(f, &template_name, &mut o)?;
    }

    Ok(())
}
