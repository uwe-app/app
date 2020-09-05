use std::env;
use std::fmt;
use std::time::Instant;

use search::config::Config;
use search::Result;

use num_format::{Locale, ToFormattedString};

const HELP_TEXT: &str = r#"
Stork 0.7.2  --  by James Little
Impossibly fast web search, made for static sites.

USAGE:
    stork --build [config.toml]
    stork --search [./index.st] "[query]"
"#;

pub type ExitCode = i32;
pub const EXIT_SUCCESS: ExitCode = 0;
pub const EXIT_FAILURE: ExitCode = 1;

fn main() {
    let mut a = Argparse::new();
    a.register("build", build_handler, 1);
    a.register_help(HELP_TEXT);
    std::process::exit(a.exec(env::args().collect()));
}

fn build_handler(args: &[String]) -> Result<()> {
    let start_time = Instant::now();
    let config = Config::from_file(std::path::PathBuf::from(&args[2]));
    let index = search::build(&config);
    let build_time = Instant::now();
    let bytes_written = search::writer::write(&index, &config.output.filename)?;
    let end_time = Instant::now();
    println!(
        "Index built, {} bytes written to {}. {}\n\t{:.3?}s to build index\n\t{:.3?}s to write file\n\t{:.3?}s total",
        bytes_written.to_formatted_string(&Locale::en),
        config.output.filename,
        {
            if bytes_written != 0 {
                ""
            } else {
                "(Maybe you're in debug mode.)"
            }
        },
        build_time.duration_since(start_time).as_secs_f32(),
        end_time.duration_since(build_time).as_secs_f32(),
        end_time.duration_since(start_time).as_secs_f32()
    );

    Ok(())
}

pub struct Argparse {
    commands: Vec<Command>,
    help_text: Option<String>,
}

struct Command {
    name: String,
    action: fn(&[String]) -> Result<()>,
    number_of_args: ValueOrRange,
}

enum ValueOrRange {
    Value(u8),
    Range(u8, u8),
}

impl fmt::Display for ValueOrRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueOrRange::Value(val) => write!(f, "{}", val),

            ValueOrRange::Range(min, max) => {
                write!(f, "between {} and {}", min, max)
            }
        }
    }
}

impl Argparse {
    pub fn new() -> Argparse {
        Argparse {
            commands: vec![],
            help_text: None,
        }
    }

    pub fn register(
        &mut self,
        cmd_name: &str,
        action: fn(&[String]) -> Result<()>,
        number_of_args: u8,
    ) {
        self.commands.push(Command {
            name: cmd_name.to_string(),
            action,
            number_of_args: ValueOrRange::Value(number_of_args),
        })
    }

    #[allow(dead_code)]
    pub fn register_range(
        &mut self,
        cmd_name: &str,
        action: fn(&[String]) -> Result<()>,
        args_range: (u8, u8),
    ) {
        let min = std::cmp::min(args_range.0, args_range.1);
        let max = std::cmp::max(args_range.0, args_range.1);
        let number_of_args = if min == max {
            ValueOrRange::Value(min)
        } else {
            ValueOrRange::Range(min, max)
        };

        self.commands.push(Command {
            name: cmd_name.to_string(),
            action,
            number_of_args,
        })
    }

    pub fn register_help(&mut self, text: &str) {
        self.help_text = Some(text.to_string());
    }

    pub fn exec(&self, args: Vec<String>) -> ExitCode {
        if args.len() < 2 || ["-h", "--help"].contains(&args[1].as_str()) {
            if let Some(help_text) = &self.help_text {
                println!("{}", help_text);
                return EXIT_SUCCESS;
            }
        }

        for command in &self.commands {
            if args[1] == ["--", &command.name].concat() {
                let number_of_args = args.len() - 2;
                let valid = match command.number_of_args {
                    ValueOrRange::Value(val) => (number_of_args as u8) == val,

                    ValueOrRange::Range(min, max) => {
                        (number_of_args as u8) >= min
                            && (number_of_args as u8) <= max
                    }
                };

                if !valid {
                    println!(
                        "Wrong number of arguments given to `{}` command. Expected {} but got {}.",
                        command.name, command.number_of_args, number_of_args
                    );
                    return EXIT_FAILURE;
                } else {
                    (command.action)(&args).expect("Command action failed");
                    return EXIT_SUCCESS;
                }
            }
        }

        println!("Command not found: `{}`.", args[1]);
        EXIT_FAILURE
    }
}
