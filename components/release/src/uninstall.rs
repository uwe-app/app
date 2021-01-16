use std::fs;
use std::io::{self, BufRead, Write};

use log::info;

use crate::Result;

/// Uninstall the program.
pub async fn uninstall() -> Result<()> {
    let dir = dirs::root_dir()?;

    if !dir.exists() || !dir.is_dir() {
        info!("Not installed {}", dir.display());
        info!("");
        info!("To install the platform tools run:");
        info!("");
        info!("curl https://releases.uwe.app/install.sh | sh");
        info!("");
        return Ok(());
    }

    print!(" Uninstall {}? (y/n) ", dir.display());
    io::stdout().flush()?;

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let response = line.unwrap();
        if response == "y" || response == "yes" {
            utils::terminal::clear_previous_line()?;
            fs::remove_dir_all(&dir)?;
            info!("Uninstalled {} ✓", dir.display());
        }
        break;
    }

    Ok(())
}
