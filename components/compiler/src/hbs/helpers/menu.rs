use std::path::PathBuf;
use std::sync::Arc;

use bracket::helper::prelude::*;
use serde_json::json;

use collator::menu::{self, PageData};

use config::{href::UrlPath, MenuEntry, MenuReference, RuntimeOptions};

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

    fn listing_menu<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
    ) -> HelperResult<MenuEntry> {
        let collation = self.context.collation.read().unwrap();

        let dir = if let Some(path) = ctx.param("path") {
            let path = path.as_str().ok_or_else(|| {
                HelperError::new(
                    "Type error for `menu` helper, hash parameter `path` must be a string")
            })?;

            let normalized_href = collation.normalize(path);
            if let Some(base_path) = collation.get_link(&normalized_href) {
                //base_path.foo();
                if let Some(page_lock) = collation.resolve(base_path.as_ref()) {
                    let page = page_lock.as_ref().read().unwrap();
                    // NOTE: we want to operate on the destination so that
                    // NOTE: url paths are more intuitive when `path="/docs/"` etc.
                    let destination = page.destination();
                    let path = destination.to_path_buf();
                    let dir = path.parent().unwrap().to_path_buf();
                    UrlPath::from(dir)
                } else {
                    panic!("Menu helper could not read page data");
                }
            } else {
                return Err(HelperError::new(format!(
                    "Type error for `menu` helper, no page for path '{}'",
                    path
                )));
            }
        } else {
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

            UrlPath::from(dir_path)
        };

        let mut depth = ctx
            .param("depth")
            .or(Some(&json!(1)))
            .and_then(|v| v.as_u64())
            .ok_or(HelperError::new(
                "Type error for `menu` helper, hash parameter `depth` must be a positive integer",
            ))?;

        if depth == 0 {
            depth = 64;
        }

        let mut include_index = ctx
            .param("include-index")
            .or(Some(&json!(false)))
            .and_then(|v| v.as_bool())
            .ok_or(HelperError::new(
                "Type error for `menu` helper, hash parameter `include-index` must be a boolean",
            ))?;

        let definition = MenuReference::Directory {
            directory: dir,
            depth: Some(depth as usize),
            description: None,
            include_index: Some(include_index),
        };

        Ok(MenuEntry::new(definition))
    }

    fn render_listing_node<'render, 'call>(
        &self,
        node: &'render Node<'render>,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
    ) -> HelperValue {
        let menu = self.listing_menu(rc, ctx)?;
        let collation = self.context.collation.read().unwrap();
        let locale = collation.locale.read().unwrap();
        let (_result, pages) =
            menu::build(&self.context.options, &*locale, &menu)
                .map_err(|e| HelperError::new(e.to_string()))?;
        self.render_pages(pages, node, rc, ctx)
    }

    fn render_listing<'render, 'call>(
        &self,
        rc: &mut Render<'render>,
        ctx: &Context<'call>,
    ) -> HelperValue {
        let menu = self.listing_menu(rc, ctx)?;
        let collation = self.context.collation.read().unwrap();
        let locale = collation.locale.read().unwrap();
        let (result, _pages) =
            menu::build(&self.context.options, &*locale, &menu)
                .map_err(|e| HelperError::new(e.to_string()))?;

        let template_path = rc
            .try_evaluate("@root/file.template", &[Type::String])?
            .as_str()
            .unwrap()
            .to_string();

        let result = rc.once(&template_path, &result.value, rc.data())?;
        rc.write(&result)?;

        Ok(None)
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
            let menus = collation.get_menus();
            let menu = menus.get(&name);
            if let Some(result) = menu {
                rc.push_scope(Scope::new());
                for href in result.pages.iter() {
                    if let Some(page_path) =
                        collation.get_link(&collation.normalize(&**href))
                    {
                        if let Some(page) =
                            collation.resolve(page_path.as_ref())
                        {
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
            self.render_listing_node(node, rc, ctx)
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
        let menus = collation.get_menus();
        let name = collation.get_menu_template_name(key);

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
        /*
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
        };
        */

        let key = ctx.try_get(0, &[Type::String])?.as_str().unwrap();
        self.render_menu_by_name(key, rc, ctx)
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

        // Handle inner template iteration
        if let Some(node) = template {
            if list {
                self.render_listing_node(node, rc, ctx)
            } else {
                self.render_template(node, rc, ctx)
            }
        // Otherwise render directly
        } else {
            if list {
                self.render_listing(rc, ctx)
            } else {
                self.render_menu(rc, ctx)
            }
        }
    }
}
