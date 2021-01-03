use std::env;

use uwe::shim::{process, Result};

fn main() -> Result<()> {
    let cwd = env::current_dir()?;
    let args: Vec<String> = env::args().skip(1).collect();

    //println!("Args {:?}", args);
    //std::process::exit(1);

    let _ = process("uwe")
        .args(args.as_slice())
        .exec_replace()?;

    Ok(())
}

