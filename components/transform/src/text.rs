use std::fmt;

#[derive(Debug, Default, Clone)]
pub struct TextExtraction {
    pub title: Option<String>,
    pub chunks: Vec<String>,
    pub words: usize,
}

impl TextExtraction {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn to_chunk_string(&self) -> String {
        return self
            .chunks
            .iter()
            // HACK: !!!
            // NOTE: this character causes a problem for search index
            // NOTE: excerpt highlighting
            .map(|c| c.replace("|", ""))
            .collect::<Vec<_>>()
            .join(" ");
    }
}

impl fmt::Display for TextExtraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref title) = self.title {
            write!(f, "{}\n\n", title)?;
        }

        for c in self.chunks.iter() {
            write!(f, "{} ", c)?;
        }

        Ok(())
    }
}
