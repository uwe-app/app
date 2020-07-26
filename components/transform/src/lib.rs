use lol_html::{element, rewrite_str, RewriteStrSettings};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Rewriting(#[from] lol_html::errors::RewritingError),
}

type Result<T> = std::result::Result<T, Error>;

pub fn textify(src: &str) -> Result<String> {

    Ok(rewrite_str(src,
        RewriteStrSettings {
            element_content_handlers: vec![
                element!("title", |el| {
                    el.remove_and_keep_content();
                    Ok(())
                }),
                element!("body *", |el| {
                    el.remove_and_keep_content();
                    Ok(())
                }),
                //text!("body *", |t| {
                    //if t.last_in_text_node() {
                        ////t.after(" world", ContentType::Text);
                    //}
                    //Ok(())
                //})
            ],
            ..RewriteStrSettings::default()
        })?
    )
}
