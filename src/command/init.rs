use std::path::PathBuf;
use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::utils;
use crate::Error;

use log::info;

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
            let pth = utils::copy_asset_bundle_file(f, common_name, &target)?;
            info!("copy {}", pth.display());
        }
        for f in template_files.iter() {
            let pth = utils::copy_asset_bundle_file(f, &template_name, &target)?;
            info!("copy {}", pth.display());
        }
    } else {
        return Err(Error::new("init target was not given".to_string()))
    }

    Ok(())
}
