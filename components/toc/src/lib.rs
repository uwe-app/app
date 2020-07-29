use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid heading tag name {0}")]
    InvalidTagName(String),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Heading {
    pub depth: usize,
    pub id: String,
    pub text: String,
}

impl Heading {
    pub fn parse(tag_name: &str, id: &str, text: &str) -> Result<Self> {

        let depth = if tag_name == "h1" { 0 }
            else if tag_name == "h2" { 1 } 
            else if tag_name == "h3" { 2 }
            else if tag_name == "h4" { 3 }
            else if tag_name == "h5" { 4 }
            else if tag_name == "h6" { 5 } 
            else {
                return Err(Error::InvalidTagName(tag_name.to_string()));
            };

        Ok(Self {
            depth,
            id: id.to_string(),
            text: text.to_string(),
        })
    }
}

#[derive(Debug)]
pub struct TableOfContents {
    pub entries: Vec<Heading>,
}

impl TableOfContents {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add(&mut self, tag_name: &str, id: &str, text: &str) -> Result<()> {
        let heading = Heading::parse(tag_name, id, text)?;
        self.entries.push(heading);
        Ok(())
    }

    pub fn to_html_string(&self, tag_name: &str, class_name: &str, from: &str, to: &str) -> Result<String> {
        let from = Heading::parse(from, "", "")?;
        let to = Heading::parse(to, "", "")?;

        let mut markup = format!("<{} class=\"{}\">", tag_name, class_name);

        markup.push_str(&format!("</{}>", tag_name));
        Ok(markup)
    }
}

