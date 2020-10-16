use std::io;

use handlebars::*;

//pub mod author;
pub mod bookmark;
pub mod crumbtrail;
pub mod date;
pub mod favicon;
pub mod feed;
pub mod include;
pub mod json;
pub mod link;
pub mod livereload;
pub mod markdown;
pub mod matcher;
pub mod menu;
pub mod page;
pub mod parent;
pub mod partial;
pub mod random;
pub mod scripts;
pub mod search;
pub mod sibling;
pub mod slug;
pub mod styles;
pub mod toc;
pub mod word;

pub struct BufferedOutput {
    buffer: String,
}

impl Output for BufferedOutput {
    fn write(&mut self, seg: &str) -> Result<(), io::Error> {
        self.buffer.push_str(seg);
        Ok(())
    }
}
