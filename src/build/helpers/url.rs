use std::path::Path;

use handlebars::*;
use serde_json::json;
use log::debug;

use crate::build::matcher;
use crate::build::loader;
use crate::BuildOptions;

use crate::{INDEX_HTML};

#[derive(Clone, Copy)]
pub struct Link;

impl HelperDef for Link{
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let base_path = rc
            .evaluate(ctx, "@root/file.source")?
            .as_json()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error for `file`, string expected"))?
            .replace("\"", "");

        let opts = rc
            .evaluate(ctx, "@root/context.options")?
            .as_json()
            .as_object()
            .ok_or_else(|| RenderError::new("Type error for `options`, map expected"))?
            .to_owned();

        let opts: BuildOptions = serde_json::from_value(json!(opts)).unwrap();
        let path = Path::new(&base_path);

        let mut input: String = "".to_string();

        if let Some(p) = h.params().get(0) {
            if !p.is_value_missing() {
                input = p.value().as_str().unwrap_or("").to_string();
            }

            if input.is_empty() {
                return Err(RenderError::new("Type error for `link`, expected string parameter")) 
            }

            let passthrough = !input.starts_with("/") || input.starts_with("http:") || input.starts_with("https:");
            if passthrough {
                out.write(&input)?;
                if opts.index_links && (input == "." || input == "..") {
                    out.write("/")?;
                    out.write(INDEX_HTML)?;
                }
                return Ok(())
            }

            // Strip the leading slash
            if input.starts_with("/") {
                input = input.trim_start_matches("/").to_owned();
            }

            let exists = matcher::source_exists(&opts.source, &input, opts.clean_url);

            if !exists {
                return Err(
                    RenderError::new(format!("Type error for `link`, missing url {}", input)))
            }

            if let Ok(rel) = path.strip_prefix(opts.source) {
                let mut value: String = "".to_string();
                if let Some(p) = rel.parent() {
                    if opts.clean_url && matcher::is_clean(&path) {
                        value.push_str("../");
                    }
                    for _ in p.components() {
                        value.push_str("../");
                    }
                }

                value.push_str(&input);
                if opts.index_links && (value.ends_with("/") || value == "") {
                    value.push_str(INDEX_HTML);
                }
                debug!("link {:?}", value);

                out.write(&value)?;

            } else {
                return Err(RenderError::new("Type error for `link`, file is outside source!")) 
            }


        } else {
            return Err(RenderError::new("Type error for `link`, expected string parameter")) 
        }
        Ok(())
    }
}


#[derive(Clone, Copy)]
pub struct Components;

impl HelperDef for Components{
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let base_path = rc
            .evaluate(ctx, "@root/file.target")?
            .as_json()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error for `dest`, string expected"))?
            .replace("\"", "");

        let opts = rc
            .evaluate(ctx, "@root/context.options")?
            .as_json()
            .as_object()
            .ok_or_else(|| RenderError::new("Type error for `options`, map expected"))?
            .to_owned();

        let opts: BuildOptions = serde_json::from_value(json!(opts)).unwrap();
        let path = Path::new(&base_path).to_path_buf();

        let template = h.template();
        match template {
            Some(t) => {

                if let Ok(rel) = path.strip_prefix(&opts.target) {
                    let mut buf = rel.to_path_buf();
                    if buf.ends_with(INDEX_HTML) {
                        buf.pop();
                    }

                    let mut parts: Vec<String> = buf.iter()
                        .map(|part| part.to_string_lossy().into_owned())
                        .collect();

                    // Add an empty string for home page
                    parts.insert(0, "/".to_string());

                    let up = "../".to_string();
                    let mut href = "".to_string();
                    for (pos, name) in parts.iter().enumerate() {
                        let amount = (parts.len() - 1) - pos;
                        let first = pos == 0;
                        let last = amount == 0;
                        if pos > 0 {
                            href.push('/');
                            href.push_str(&name);
                        }
                        let mut url = up.repeat(amount);
                        if opts.index_links {
                            url.push_str(INDEX_HTML);
                        }

                        if let Some(src) = matcher::lookup(
                            &opts.source, &href, opts.clean_url) {
                            let mut data = loader::compute(src);
                            data.insert("first".to_string(), json!(first));
                            data.insert("last".to_string(), json!(last));
                            data.insert("href".to_string(), json!(url));
                            let mut local_rc = rc.clone();
                            let local_ctx = Context::wraps(&data)?;
                            t.render(r, &local_ctx, &mut local_rc, out)?;
                        }
                    }
                }


            }
            None => return Err(RenderError::new("Template expected for components helper")),
        }

        Ok(())
    }
}


#[derive(Clone, Copy)]
pub struct Match;

impl HelperDef for Match{
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        _r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {

        let base_path = rc
            .evaluate(ctx, "@root/file.target")?
            .as_json()
            .as_str()
            .ok_or_else(|| RenderError::new("Type error for `dest`, string expected"))?
            .replace("\"", "");

        let opts = rc
            .evaluate(ctx, "@root/context.options")?
            .as_json()
            .as_object()
            .ok_or_else(|| RenderError::new("Type error for `options`, map expected"))?
            .to_owned();

        let opts: BuildOptions = serde_json::from_value(json!(opts)).unwrap();
        let path = Path::new(&base_path).to_path_buf();

        if h.params().len() != 2 && h.params().len() != 3 {
            return Err(RenderError::new("Type error for `match`, two parameters expected"))
        }

        let mut target: String = "".to_owned();
        let mut output: String = "".to_owned();
        let mut exact: bool = false;

        if let Some(p) = h.params().get(0) {
            if !p.is_value_missing() {
                target= p.value().as_str().unwrap_or("").to_string();
            }
        }

        if target.ends_with("/") {
            target = target.trim_end_matches("/").to_string();
        }

        if let Some(p) = h.params().get(1) {
            if !p.is_value_missing() {
                output = p.value().as_str().unwrap_or("").to_string();
            }
        }

        if let Some(p) = h.params().get(2) {
            if !p.is_value_missing() {
                exact = p.value().as_bool().unwrap_or(true);
            }
        }

        if let Ok(rel) = path.strip_prefix(&opts.target) {
            let mut pth = "".to_string();
            pth.push('/');
            pth.push_str(&rel.to_string_lossy().into_owned());
            if pth.ends_with(INDEX_HTML) {
                pth = pth.trim_end_matches(INDEX_HTML).to_string();
            }
            if pth.ends_with("/") {
                pth = pth.trim_end_matches("/").to_string();
            }

            let matches = (exact && pth == target)
                || (!exact && target != "" && pth.starts_with(&target))
                || (!exact && target == "" && pth == "");

            if matches {
                out.write(&output)?;
            }
        }
        Ok(())
    }
}
