use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../editor/build/release"]
pub struct Asset;
