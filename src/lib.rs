use log::{info,error,debug,trace};
use std::io;
use std::path::Path;
use std::path::PathBuf;
use walkdir::{WalkDir,DirEntry};

use mdbook::MDBook;

pub mod matcher;
pub mod parser;
pub mod renderer;

use matcher::{FileType};

pub struct InputOptions {
    pub source: PathBuf,
    pub follow_links: bool,
    pub matcher: matcher::FileMatcher,
}

pub struct OutputOptions {
    pub target: PathBuf,
    pub theme: String,
}

impl OutputOptions {

    // Build the destination file path.
    pub fn destination(&self, input: &PathBuf, file: &PathBuf, file_type: &FileType) -> PathBuf {
        let relative = file.strip_prefix(input);
        match relative {
            Ok(relative) => {
                let mut result = self.target.join(relative).to_owned();
                match file_type {
                    FileType::Markdown | FileType::Handlebars => {
                        result.set_extension("html");
                    },
                    _ => {}
                }
                result
            },
            Err(e) => panic!(e),
        }
    }

}

pub struct Finder {
    input: InputOptions,
    output: OutputOptions,
}

impl Finder {

    pub fn new(input: InputOptions, output: OutputOptions) -> Self {
       Finder{input, output} 
    }

    fn process(&self, input: PathBuf, file_type: FileType) -> io::Result<()> {
        let mut parser = parser::Parser::new();
        let mut renderer = renderer::Renderer::new();

        let output = self.output.destination(&self.input.source, &input, &file_type);

        match file_type {
            FileType::Unknown => {
                return renderer.copy(input, output)
            },
            FileType::Html | FileType::Handlebars => {
                info!("HTML {} -> {}", input.display(), output.display());
                let result = parser.parse_html(input);
                match result {
                    Ok(s) => {
                        trace!("{}", s);
                        return renderer.write_string(output, s)
                    },
                    Err(e) => return Err(e)
                }
            },
            FileType::Markdown => {
                info!("MARK {} -> {}", input.display(), output.display());
                let result = parser.parse_markdown(input);
                match result {
                    Ok(s) => {
                        trace!("{}", s);
                        return renderer.write_string(output, s)
                    },
                    Err(e) => return Err(e)
                }
            },
            FileType::Ignored => {
                debug!("NOOP {}", input.display());
            },
            FileType::Private => {
                // Ignore templates here as they are located and 
                // used during the parsing and rendering process
                debug!("PRIV {}", input.display());
            },
        }

        Ok(())
    }

    fn copy_book(&self, source_dir: &Path, build_dir: PathBuf) {
        let renderer = renderer::Renderer::new();

        // Jump some hoops to bypass the book build_dir
        let relative = source_dir.strip_prefix(&self.input.source).unwrap();
        let mut base = self.output.target.clone();
        base.push(relative);

        //println!("dir {}", source_dir.display());
        //println!("build_dir {}", build_dir.display());
        //println!("base {}", base.display());
        

        let walker = WalkDir::new(&build_dir)
            .follow_links(self.input.follow_links);
        for entry in walker {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                let file = entry.path().to_path_buf();
                // Get a relative file and append it to the correct output base directory
                let dest = file.strip_prefix(&build_dir).unwrap();
                let mut output = base.clone();
                output.push(dest);
                // Copy the file content
                let copied = renderer.copy(file, output);
                match copied {
                    Err(e) => {
                        error!("{}", e);
                        std::process::exit(1);
                    },
                    _ => {}
                }
            }
        }
    }

    fn book(&self, dir: &Path) {
        info!("BOOK {}", dir.display());

        let result = MDBook::load(dir);
        match result {
            Ok(mut md) => {
                //println!("{:?}", md.config);

                md.config.set("output.html.theme", &self.output.theme);
                let theme = md.config.get("output.html.theme").unwrap();

                debug!("THEME {}", theme);

                let built = md.build();
                match built {
                    Ok(_) => {
                        // TODO: copy dir/BOOK -> target output directory
                        let bd = md.config.build.build_dir;
                        let mut src = dir.to_path_buf();
                        src.push(bd);
                        self.copy_book(dir, src);
                    },
                    Err(e) => {
                        error!("{}", e);
                        std::process::exit(1);
                    },
                }
            },
            Err(e) => {
                error!("{}", e);
                std::process::exit(1);
            },
        }
    }

    fn handle(&self, entry: &DirEntry) -> bool {
        let path = entry.path().clone();
        if path.is_dir() {
            let mut book = path.to_path_buf();
            book.push("book.toml");
            if book.exists() {
                self.book(book.parent().unwrap());
                return false
            }
        }
        true
    }

    // Find files in an input directory to process and invoke the callback 
    // for each matched file.
    fn walk<T>(&self, callback: T) where T: Fn(PathBuf, FileType) {
        let walker = WalkDir::new(self.input.source.clone())
            .follow_links(self.input.follow_links)
            .into_iter();
        for entry in walker.filter_entry(|e| self.handle(e)) {
            let entry = entry.unwrap();
            if entry.file_type().is_file() {
                let file = entry.path().to_path_buf();
                let file_type = self.input.matcher.get_type(&file);
                callback(file, file_type)
            }
        }
    }

    // Find files and process each entry.
    pub fn run(&self) {
        self.walk(|file: PathBuf, file_type: FileType| {
            let result = self.process(file, file_type);
            match result {
                Err(e) => {
                    error!("{}", e);
                    std::process::exit(1);
                },
                _ => {},
            }
        });
    }
}
