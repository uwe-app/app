use std::path::PathBuf;
use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::utils;
use crate::Error;

use log::info;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "template"]
struct Asset;

#[derive(Debug)]
pub struct InitOptions {
    pub target: Option<PathBuf>,
    pub template: String,
    pub list: bool,
}

lazy_static! {
    #[derive(Debug)]
    pub static ref DATA: Mutex<BTreeMap<String, String>> = {
        let mut map = BTreeMap::new();
        map.insert("newcss".to_string(), "https://github.com/xz/new.css".to_string());
        map.insert("tacit".to_string(), "https://github.com/yegor256/tacit".to_string());
        map.insert("bahunya".to_string(), "https://github.com/Kimeiga/bahunya".to_string());
        Mutex::new(map)
    };
}

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

    let data = DATA.lock().unwrap();
    let template_name = options.template;

    if options.list {
        for (k, v) in data.iter() {
            info!("{} {}", k, v);
        }
        return Ok(())
    }

    if !data.contains_key(&template_name) {
        return Err(Error::new(format!("unknown template {}", &template_name)))
    }

    //if !TEMPLATES.contains(&template_name.as_str()) {
        //return Err(Error::new(format!("unknown template {}", &template_name)))
    //}

    let common_name = "common";

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

    if let Some(target) = options.target {

        if target.exists() {
            return Err(
                Error::new(format!("directory already exists: {}", target.display())));
        }


        info!("init {} using {}", target.display(), template_name);

        for f in common_files.iter() {
            let mut o = target.clone();
            copy_file(f, common_name, &mut o)?;
        }
        for f in template_files.iter() {
            let mut o = target.clone();
            copy_file(f, &template_name, &mut o)?;
        }
    } else {
        return Err(Error::new("init target was not given".to_string()))
    }

    Ok(())
}
