use log::{info,error,debug,trace};
use std::io;
use std::path::Path;
use std::path::PathBuf;
use walkdir::{WalkDir,DirEntry};

use mdbook::MDBook;

pub mod matcher;
pub mod parser;
pub mod fs;
pub mod template;

use matcher::{FileType};

pub struct InputOptions {
    pub matcher: matcher::FileMatcher,
    pub source: PathBuf,
    pub follow_links: bool,
    pub layout: String,
    pub template: String,
}

pub struct OutputOptions {
    pub matcher: matcher::FileMatcher,
    pub target: PathBuf,
    pub theme: String,
    pub clean: bool,
}

impl OutputOptions {

    // Build the destination file path.
    pub fn destination(&self, input: &PathBuf, file: &PathBuf, file_type: &FileType) -> PathBuf {
        let relative = file.strip_prefix(input);
        match relative {
            Ok(relative) => {
                let mut result = self.target.join(relative).to_owned();
                match file_type {
                    FileType::Markdown | FileType::Html => {
                        result.set_extension("html");

                        let clean_target = file.clone();
                        if self.clean && !self.matcher.is_index(&clean_target) {
                            if let Some(parent) = clean_target.parent() {
                                if let Some(stem) = clean_target.file_stem() {
                                    let mut target = parent.to_path_buf();
                                    target.push(stem);
                                    target.push(self.matcher.get_index_stem());

                                    //println!("{:?}", target);

                                    // No corresponding input file that would collide
                                    // with the clean output destination
                                    if !self.matcher.has_parse_file(&target) {
                                        //println!("{:?}", target); 
                                        //println!("{:?}", result); 
                                        let clean_result = result.clone();
                                        if let Some(parent) = clean_result.parent() {
                                            if let Some(stem) = clean_result.file_stem() {
                                                let mut res = parent.to_path_buf();
                                                res.push(stem);
                                                res.push(self.matcher.get_index_stem());
                                                res.set_extension("html");
                                                debug!("clean url {:?}", res); 
                                                result = res;
                                            }
                                        }
                                    }
                                }
                            }

                        }
                    },
                    _ => {}
                }
                result
            },
            Err(e) => panic!(e),
        }
    }

}


fn process_file(
    parser: &mut parser::Parser,
    input: &InputOptions,
    output: &OutputOptions,
    file: PathBuf,
    file_type: FileType) -> io::Result<()> {

    let output = output.destination(&input.source, &file, &file_type);

    match file_type {
        FileType::Unknown => {
            return fs::copy(file, output)
        },
        FileType::Html => {
            info!("html {} -> {}", file.display(), output.display());
            let result = parser.parse_html(file);
            match result {
                Ok(s) => {
                    trace!("{}", s);
                    return fs::write_string(output, s)
                },
                Err(e) => return Err(e)
            }
        },
        FileType::Markdown => {
            info!("mark {} -> {}", file.display(), output.display());
            let result = parser.parse_markdown(file);
            match result {
                Ok(s) => {
                    trace!("{}", s);
                    return fs::write_string(output, s)
                },
                Err(e) => return Err(e)
            }
        },
        FileType::Ignored | FileType::Private | FileType::Template => {
            // Ignore templates here as they are located and 
            // used during the parsing and rendering process
            debug!("noop {}", file.display());
        },
    }

    Ok(())
}

pub struct Finder {
    input: InputOptions,
    output: OutputOptions,
}

impl Finder {

    pub fn new(input: InputOptions, output: OutputOptions) -> Self {
        Finder{input, output} 
    }

    fn copy_book(&self, source_dir: &Path, build_dir: PathBuf) {

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
                let copied = fs::copy(file, output);
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
        info!("book {}", dir.display());

        let result = MDBook::load(dir);
        match result {
            Ok(mut md) => {
                //println!("{:?}", md.config);

                let mut theme = self.output.theme.clone();

                if theme.is_empty() {
                    let theme_dir = self.input.matcher.get_theme_dir(&self.input.source);
                    if theme_dir.exists() {
                        if let Some(s) = theme_dir.to_str() {
                            theme = s.to_string();
                        } 
                    }
                }

                md.config.set("output.html.theme", theme);
                let theme = md.config.get("output.html.theme").unwrap();

                debug!("theme {}", theme);

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
            let buf = &path.to_path_buf();
            // Can prevent recursing if a directory pattern matches
            if self.input.matcher.is_excluded(buf) {
                return false 
            }

            if self.input.matcher.is_theme(&self.input.source, buf) {
                return false
            }
            let mut book = buf.clone();
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
    fn walk<T>(&self, mut callback: T) where T: FnMut(PathBuf, FileType) {
        let walker = WalkDir::new(self.input.source.clone())
            .follow_links(self.input.follow_links)
            .into_iter();

        let iter = walker.filter_entry(|e| self.handle(e));
        for entry in iter {
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
        // Parser must exist for the entire lifetime so that
        // template partials can be found
        let mut parser = parser::Parser::new(self.input.layout.clone());
        let mut templates = self.input.source.clone();
        templates.push(&self.input.template);
        if let Err(e) = parser.handlebars.register_templates_directory(".hbs", templates.as_path()) {
            error!("{}", e);
            std::process::exit(1);
        }

        self.walk(|file, file_type| {
            let result = process_file(&mut parser, &self.input, &self.output, file, file_type);
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
