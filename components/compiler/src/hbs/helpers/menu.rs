use std::path::PathBuf;
use std::sync::Arc;

use handlebars::*;
use serde_json::json;

use collator::{
    menu::{self, PageData},
    Collate,
};

use config::{MenuEntry, MenuReference};

use crate::BuildContext;

#[derive(Clone)]
pub struct Menu {
    pub context: Arc<BuildContext>,
}

impl Menu {
    /// Iterate the pages and render an inner block template
    /// for each page.
    fn render_pages<'reg: 'rc, 'rc>(
        &self,
        template: &'reg Template,
        pages: PageData<'_>,
        _h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let page_href = rc
            .evaluate(ctx, "@root/href")?
            .as_json()
            .as_str()
            .ok_or_else(|| {
                RenderError::new("Type error for `href`, string expected")
            })?
            .to_string();

        let block_context = BlockContext::new();
        rc.push_block(block_context);
        for (_path, href, page) in pages.iter() {
            let li = &*page.read().unwrap();
            let is_self = href == &page_href;
            if let Some(ref mut block) = rc.block_mut() {
                block.set_local_var("@self".to_string(), json!(is_self));
                block.set_base_value(json!(li));
            }
            template.render(r, ctx, rc, out)?;
        }
        rc.pop_block();

        Ok(())
    }

    fn list_parent_pages<'reg: 'rc, 'rc>(
        &self,
        template: &'reg Template,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        let base_path = rc
            .evaluate(ctx, "@root/file.source")?
            .as_json()
            .as_str()
            .ok_or_else(|| {
                RenderError::new(
                    "Type error for `file.source`, string expected",
                )
            })?
            .to_string();

        let path = PathBuf::from(&base_path);
        let dir = path.parent().unwrap().to_path_buf();

        let dir_path = dir
            .strip_prefix(&self.context.options.source)
            .unwrap()
            .to_string_lossy()
            .to_owned()
            .to_string();

        let definition = MenuReference::Directory {
            directory: dir_path,
            depth: Some(1),
            description: None,
        };

        let menu = MenuEntry::new_reference(definition);

        let collation = self.context.collation.read().unwrap();

        let (_result, pages) =
            menu::build(&self.context.options, &collation.locale, &menu)
                .map_err(|e| RenderError::new(e.to_string()))?;

        self.render_pages(template, pages, h, r, ctx, rc, out)
    }

    /// Render an inner template block.
    fn render_template<'reg: 'rc, 'rc>(
        &self,
        template: &'reg Template,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        // TODO: handle dynamically rendering inner templates
        // TODO: from helper parameters!!!

        self.list_parent_pages(template, h, r, ctx, rc, out)
    }

    /// Render a menu reference by name.
    fn render_menu_by_name<'reg: 'rc, 'rc>(
        &self,
        key: &str,
        _h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        _rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        // TODO: handle file-specific menu overrides

        let collation = self.context.collation.read().unwrap();
        let menus = collation.get_graph().get_menus();
        let name = menus.get_menu_template_name(key);

        if let Some(_tpl) = r.get_template(&name) {
            let result = r.render_with_context(&name, ctx)?;
            out.write(&result)?;
        }

        Ok(())
    }

    /// Render a menu reference.
    fn render_menu<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        // Render the MENU.md folder convention
        let key: String = if h.params().is_empty() {
            let source_path = rc
                .evaluate(ctx, "@root/file.source")?
                .as_json()
                .as_str()
                .ok_or_else(|| {
                    RenderError::new(
                        "Type error for `file.source`, string expected",
                    )
                })?
                .to_string();

            let path = PathBuf::from(&source_path);

            if let Some(parent) = path.parent() {
                parent.to_string_lossy().into_owned()
            } else {
                source_path
            }

        // Render a named argument
        } else {
            h.params()
                .get(0)
                .ok_or_else(|| {
                    RenderError::new(
                        "Type error in `menu`, expected parameter at index 0",
                    )
                })?
                .value()
                .as_str()
                .ok_or_else(|| {
                    RenderError::new(
                        "Type error in `menu`, expected string parameter at index 0",
                    )
                })?.to_string()
        };

        self.render_menu_by_name(&key, h, r, ctx, rc, out)
    }
}

impl HelperDef for Menu {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &Helper<'reg, 'rc>,
        r: &'reg Handlebars<'_>,
        ctx: &'rc Context,
        rc: &mut RenderContext<'reg, 'rc>,
        out: &mut dyn Output,
    ) -> HelperResult {
        if let Some(template) = h.template() {
            self.render_template(template, h, r, ctx, rc, out)
        } else {
            self.render_menu(h, r, ctx, rc, out)
        }
    }
}
