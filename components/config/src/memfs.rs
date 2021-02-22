use dyn_clone::DynClone;
use std::borrow::Cow;

/// Type for implementations that expose in-memory file systems.
///
/// Based on the `rust-embed` API that we can easily wrap embedded file systems
/// that can then be passed around in virtual host configurations for mounting
/// by the web server.
pub trait MemoryFileSystem : std::fmt::Debug {
    fn get(&self, file_path: &str) -> Option<Cow<'static, [u8]>>;
    fn iter(&self) -> Box<dyn Iterator<Item = Cow<'static, str>>>;
}

pub trait EmbeddedFileSystem : MemoryFileSystem + Send + DynClone {}

dyn_clone::clone_trait_object!(EmbeddedFileSystem);
