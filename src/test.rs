use log::info;

use crate::{
    opts::{self, Test},
    Error, Result,
};

pub async fn run(opts: Test) -> Result<()> {
    println!("Running tests...");
    Ok(())
}
