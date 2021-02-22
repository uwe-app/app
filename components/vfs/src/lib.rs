use rust_embed::RustEmbed;
use std::borrow::Cow;

use config::memfs::{MemoryFileSystem, EmbeddedFileSystem};

#[derive(RustEmbed)]
#[folder = "../../editor/build/release"]
pub struct Editor;

#[derive(Debug, Clone)]
pub struct EditorFileSystem;

/// Wrapper for the rust embed API so we can pass around a
/// trait type used by the web server to mount in-memory file systems.
impl MemoryFileSystem for EditorFileSystem {
    fn get(&self, file_path: &str) -> Option<Cow<'static, [u8]>> {
        Editor::get(file_path)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = Cow<'static, str>>> {
        Box::new(Editor::iter())
    }
}

impl EmbeddedFileSystem for EditorFileSystem {}

/// Get the in-memory file system for the editor assets.
pub fn editor() -> Box<dyn EmbeddedFileSystem> {
    Box::new(EditorFileSystem {})
}
