extern crate tar;

use std::path::Path;

use std::io::prelude::*;
use std::fs::File;
use tar::Builder;

use crate::{Result, walk};

pub async fn pack<S: AsRef<Path>, D: AsRef<Path>>(source: S, dest: D) -> Result<()> {
    let src = source.as_ref().to_path_buf();
    let file = File::create(dest)?;
    let mut tarball = Builder::new(file);

    let mut files = walk::find(source, |_| true);
    for file in files.into_iter() {
        let rel = file.strip_prefix(&src)?;
        println!("Got tarball file {} {}", rel.display(), file.display());
        //tarball.append_file(rel, &mut File::open(&file)?)?;
    }

    tarball.into_inner()?;

    Ok(())
}
