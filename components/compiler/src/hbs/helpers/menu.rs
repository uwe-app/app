use std::path::PathBuf;
use std::sync::Arc;

use bracket::helper::prelude::*;
use serde_json::json;

use collator::{
    menu::{self, PageData},
    Collate, LinkCollate,
};

use config::{href::UrlPath, MenuEntry, MenuReference};

use crate::BuildContext;

pub struct Menu {
    pub context: Arc<BuildContext>,
}

impl Menu {
    /// Iterate the pages and render an inner block template
    /// for each page.
    fn render_pages<'render, 'call>(
        &self,
        pages: PageData<'_>,
        node: &'render Node<'render>,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
    ) -> HelperValue {
        let page_href = rc
            .try_evaluate("@root/href", &[Type::String])?
            .as_str()
            .unwrap()
            .to_string();

        rc.push_scope(Scope::new());
        for (_path, href, page) in pages.iter() {
            let li = &*page.read().unwrap();
            let is_self = href == &page_href;
            if let Some(ref mut block) = rc.scope_mut() {
                block.set_local("self", json!(is_self));
                block.set_base_value(json!(li));
            }
            rc.template(node)?;
        }
        rc.pop_scope();
        Ok(None)
    }

    fn render_listing<'render, 'call>(
        &self,
        node: &'render Node<'render>,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
    ) -> HelperValue {
        let base_path = rc
            .try_evaluate("@root/file.source", &[Type::String])?
            .as_str()
            .unwrap();

        let path = PathBuf::from(&base_path);
        let dir = path.parent().unwrap().to_path_buf();

        let dir_path = dir
            .strip_prefix(&self.context.options.source)
            .unwrap()
            .to_string_lossy()
            .to_owned()
            .to_string();

        let definition = MenuReference::Directory {
            directory: UrlPath::from(dir_path),
            depth: Some(1),
            description: None,
        };

        let menu = MenuEntry::new_reference(definition);

        let collation = self.context.collation.read().unwrap();

        let (_result, pages) =
            menu::build(&self.context.options, &collation.locale, &menu)
                .map_err(|e| HelperError::new(e.to_string()))?;

        self.render_pages(pages, node, rc, ctx)
    }

    /// Render an inner template block.
    fn render_template<'render, 'call>(
        &self,
        node: &'render Node<'render>,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
    ) -> HelperValue {
        if let Some(name) = ctx.get(0) {
            let page_href = rc
                .try_evaluate("@root/href", &[Type::String])?
                .as_str()
                .unwrap()
                .to_string();

            let name = name
                .as_str()
                .ok_or(
                    HelperError::new(
                        "Type error in `menu`, expected string parameter at index 0")
                )?
                .to_string();

            let collation = self.context.collation.read().unwrap();
            let menus = collation.get_graph().get_menus();
            let menu = menus.find_result(&name);
            if let Some(result) = menu {
                rc.push_scope(Scope::new());
                for href in result.pages.iter() {
                    if let Some(page_path) =
                        collation.get_link(&collation.normalize(&**href))
                    {
                        if let Some(page) = collation.resolve(page_path) {
                            let li = &*page.read().unwrap();
                            let is_self = &**href == &page_href;
                            if let Some(ref mut block) = rc.scope_mut() {
                                block.set_local("self", json!(is_self));
                                block.set_base_value(json!(li));
                            }
                            rc.template(node)?;
                        }
                    }
                }

                rc.pop_scope();
            } else {
                return Err(HelperError::new(format!(
                    "Type error in `menu`, no menu found for {}",
                    name
                )));
            }

            Ok(None)
        } else {
            self.render_listing(node, rc, ctx)
        }
    }

    /// Render a menu reference by name.
    fn render_menu_by_name<'render, 'call>(
        &self,
        key: &str,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
    ) -> HelperValue {
        // TODO: handle file-specific menu overrides

        let collation = self.context.collation.read().unwrap();
        let menus = collation.get_graph().get_menus();
        let name = menus.get_menu_template_name(key);

        if let Some(tpl) = rc.get_template(&name) {
            rc.template(tpl.node())?;
        }

        Ok(None)
    }

    /// Render a menu reference.
    fn render_menu<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
    ) -> HelperValue {
        // Render the MENU.md folder convention
        let key: String = if ctx.arguments().is_empty() {
            let source_path = rc
                .try_evaluate("@root/file.source", &[Type::String])?
                .as_str()
                .unwrap();

            let path = PathBuf::from(&source_path);

            if let Some(parent) = path.parent() {
                parent.to_string_lossy().into_owned()
            } else {
                source_path.to_string()
            }

        // Render a named argument
        } else {
            ctx.try_get(0, &[Type::String])?
                .as_str()
                .unwrap()
                .to_string()
        };

        self.render_menu_by_name(&key, rc, ctx)
    }
}

impl Helper for Menu {
    fn call<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
        template: Option<&'render Node<'render>>,
    ) -> HelperValue {
        let list = ctx
            .param("list")
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(HelperError::new(
                "Type error for `menu` helper, hash parameter `list` must be a boolean",
            ))?;

        if let Some(node) = template {
            // Explicitly requested a directory listing
            if list {
                self.render_listing(node, rc, ctx)
            // Otherwise try to find a menu
            } else {
                self.render_template(node, rc, ctx)
            }
        } else {
            self.render_menu(rc, ctx)
        }
    }
}
