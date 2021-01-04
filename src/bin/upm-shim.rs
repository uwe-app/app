use uwe::shim::{fork, Result};

fn main() -> Result<()> {
    uwe::opts::panic_hook();
    uwe::opts::log_level("error").expect("Unable to set log level");

    let name: &str = "upm";
    if let Err(e) = fork(name, None) {
        log::error!("{}", e.to_string());
        std::process::exit(1);
    }
    Ok(())
}
