use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::convert::AsRef;

use ignore::WalkBuilder;

use handlebars::*;

#[derive(Debug)]
struct TocEntry {
    source: PathBuf,
}

fn get_files<P: AsRef<Path>>(p: P) -> io::Result<Vec<TocEntry>> {

    let mut entries: Vec<TocEntry> = Vec::new();

    for result in WalkBuilder::new(p.as_ref()).max_depth(Some(1)).build() {

        match result {
            Ok(entry) => {
                println!("got entry {:?}", entry.path());
                let path = entry.path();
                let mut matched = false;

                //if path == p.as_ref() {
                    //println!("got same path!Q!!") 
                    ////continue;
                //}

                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "md" || ext == "html" {
                            println!("FOUND MATCH");
                            //entries.push(path.to_path_buf()); 
                            matched = true;
                        } 
                    }
                } else {
                    // TODO
                }

                if matched {
                    let e = TocEntry{
                        source: path.to_path_buf(),
                    };
                    entries.push(e);
                }
            }, Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, e));
            },
        }
    }

    Ok(entries)
}

// implement by a structure impls HelperDef
#[derive(Clone, Copy)]
pub struct Toc;

impl HelperDef for Toc {
  fn call<'reg: 'rc, 'rc>(&self, h: &Helper, _: &Handlebars, ctx: &Context, rc: &mut RenderContext, out: &mut dyn Output) -> HelperResult {

  println!("template name {:?}", rc.get_current_template_name());
  println!("template name {:?}", ctx.data());

    let data = ctx.data();

    if let Some(fp) = data.get("filepath") {
        if let Some(fp) = fp.as_str() {
            let path = Path::new(&fp);
            println!("got file path {:?}", path); 
            if let Some(parent) = path.parent() {
                let entries = get_files(parent);
                println!("got paretn path {:?}", entries); 

            }
        }
    }

    //h.template()
        //.map(|t| t.render(r, ctx, rc, out))
        //.unwrap_or(Ok(()))

    Ok(())
  }
}
