use handlebars::*;

use crate::BuildContext;
use serde_json::from_value;

use config::Author;

#[derive(Clone, Copy)]
pub struct AuthorMeta<'a> {
    pub context: &'a BuildContext,
}

impl HelperDef for AuthorMeta<'_> {
    fn call<'reg: 'rc, 'rc>(
        &self,
        _h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let authors = ctx
            .data()
            .as_object()
            .ok_or_else(|| {
                RenderError::new("Type error for `author`, invalid page data")
            })
            .unwrap()
            .get("authors");

        if let Some(authors) = authors {
            if let Some(list) = authors.as_array() {
                for a in list.iter() {
                    let author: Author = from_value(a.clone()).unwrap();
                    if let Some(ref name) = author.name {
                        let content = if let Some(url) = author.url {
                            format!("{} <{}>", name, url)
                        } else {
                            format!("{}", name)
                        };

                        let markup = format!(
                            "<meta name=\"author\" content=\"{}\">",
                            utils::entity::escape(&content)
                        );
                        out.write(&markup)?;
                    }
                }
            }
        }

        Ok(())
    }
}
