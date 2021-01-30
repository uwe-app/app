use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::BuildContext;
use bracket::helper::prelude::*;

//use log::debug;

/// Helper function to get the URL for a page href.
fn url<'render, 'call>(
    rc: &mut Render<'render>,
    _ctx: &Context<'call>,
    context: &BuildContext,
    mut input: &str,
) -> HelperResult<(String, Option<PathBuf>)> {
    let abs = rc
        .evaluate("@root/absolute")?
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let base_path = rc
        .try_evaluate("@root/file.source", &[Type::String])?
        .as_str()
        .unwrap();

    let opts = &context.options;
    let path = Path::new(base_path);

    let link_config = context.config.link.as_ref().unwrap();
    let include_index = opts.settings.should_include_index();
    let make_relative =
        !abs && link_config.relative.is_some() && link_config.relative.unwrap();

    //let passthrough = !input.starts_with("/")

    let passthrough = input.starts_with(".")
        || input.starts_with("http:")
        || input.starts_with("https:");

    if passthrough {
        let mut output = input.to_string();
        if include_index && (input == "." || input == "..") {
            output.push('/');
            output.push_str(config::INDEX_HTML);
        }
        return Ok((output, None));
    }

    // Strip the leading slash
    if input.starts_with("/") {
        input = input.trim_start_matches("/");
    }

    let mut base = opts.source.clone();

    let collation = context.collation.read().unwrap();
    let normalized_href = collation.normalize(&input);

    let page_key: Option<PathBuf> = collation.find_link(&normalized_href);

    if let Some(verify) = link_config.verify {
        if verify {
            //println!("Link testing page key {:?} {} {}",
            //page_key, &normalized_href,  &input);

            if page_key.is_none() {
                return Err(HelperError::new(format!(
                    "Type error for `link`, missing url {} in {}",
                    input,
                    rc.template_name(),
                )));
            }
        }
    }

    if let Some(ref href_path) = opts.settings.base_href {
        base.push(href_path);
        if input.starts_with(href_path) {
            input = input.trim_start_matches(href_path);
            input = input.trim_start_matches("/");
        }
    }

    let (page_path, fragment) = opts.fragment(&input);

    let mut value = if make_relative {
        if let Ok(val) = opts.relative(page_path, path, base) {
            val
        } else {
            return Err(HelperError::new(
                "Type error for `link`, file is outside source!",
            ));
        }
    } else {
        format!("/{}", input)
    };

    if let Some(fragment) = fragment {
        value.push('#');
        value.push_str(fragment);
    }

    Ok((value, page_key))
}

pub struct WikiLink {
    pub context: Arc<BuildContext>,
}

impl Helper for WikiLink {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        _template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.arity(3..3)?;

        let href = ctx.try_get(0, &[Type::String])?.as_str().unwrap();

        let mut label = ctx
            .try_get(1, &[Type::String])?
            .as_str()
            .unwrap()
            .to_string();

        let mut title = ctx
            .try_get(2, &[Type::String])?
            .as_str()
            .unwrap()
            .to_string();

        let (value, page_key) = url(rc, ctx, &*self.context, href)?;

        let collation = self.context.collation.read().unwrap();

        if let Some(ref page_key) = page_key {
            if let Some(page_lock) = collation.resolve(page_key) {
                //let page_lock = page_lock.as_ref();
                let page = &*page_lock.read().unwrap();
                if label.is_empty() {
                    if let Some(ref page_label) = page.label {
                        label = page_label.clone();
                    } else if let Some(ref page_title) = page.title {
                        label = page_title.clone();
                    }
                }
                if title.is_empty() {
                    if let Some(ref page_title) = page.title {
                        title = page_title.clone();
                    }
                }
            }
        }

        if label.is_empty() {
            label = href.to_string();
        }

        if title.is_empty() {
            title = label.to_string();
        }

        // TODO: check context and write out markdown in markdown files???
        let link = format!(
            r#"<a href="{}" title="{}">{}</a>"#,
            rc.escape(&value),
            rc.escape(&title),
            rc.escape(&label)
        );

        rc.write(&link)?;
        Ok(None)
    }
}

/// Generate a link to a site page or absolute URL.
pub struct Link {
    pub context: Arc<BuildContext>,
}

impl Helper for Link {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        ctx.assert_statement(template)?;
        ctx.arity(1..1)?;
        let input = ctx.try_get(0, &[Type::String])?.as_str().unwrap();
        let (value, _) = url(rc, ctx, &*self.context, input)?;
        rc.write(&value)?;
        Ok(None)
    }
}
