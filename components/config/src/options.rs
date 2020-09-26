use std::collections::HashMap;
use std::path::{Path, PathBuf};

use globset::GlobMatcher;

use url::Url;

use crate::{
    dependency::Dependency, dependency::DependencyMap, Config, Error,
    ProfileSettings, RenderTypes, Result, TemplateEngine, HTML, INDEX_STEM,
};

#[derive(Debug, Clone)]
pub enum FileType {
    Markdown,
    Template,
    Unknown,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeOptions {
    // Project root
    pub project: PathBuf,
    // Root for the input source files
    pub source: PathBuf,
    // Root of the output
    pub output: PathBuf,
    // Target output directory including a build tag
    pub base: PathBuf,
    // The computed profile to use
    pub settings: ProfileSettings,

    // Computed plugins
    pub plugins: Option<DependencyMap>,

    // Cache of plugin dependencies that should be applied to pages
    pub styles_cache: Vec<Dependency>,
    pub scripts_cache: Vec<Dependency>,
    pub layouts_cache: HashMap<String, Vec<GlobMatcher>>,
}

impl RuntimeOptions {
    pub fn new(
        project: PathBuf,
        source: PathBuf,
        base: PathBuf,
        settings: ProfileSettings,
    ) -> Self {
        Self {
            project,
            source,
            output: settings.target.clone(),
            base,
            settings,
            plugins: None,
            styles_cache: Vec::new(),
            scripts_cache: Vec::new(),
            layouts_cache: HashMap::new(),
        }
    }

    // FIXME: stricter error handling on mismatch
    pub fn prepare(&mut self, engine: &TemplateEngine) -> Result<()> {
        if let Some(ref mut plugins) = self.plugins {
            for (_, dep) in plugins.to_vec() {
                let plugin = dep.plugin.as_ref().unwrap();
                if let Some(ref apply) = dep.apply {
                    let assets_href_base = format!(
                        "/{}",
                        utils::url::to_href_separator(plugin.assets())
                    );

                    if plugin.styles.is_some() && !apply.styles_match.is_empty()
                    {
                        let mut dep = dep.clone();
                        let styles = dep
                            .plugin
                            .as_mut()
                            .unwrap()
                            .styles
                            .as_mut()
                            .unwrap();
                        // Make style paths relative to the plugin asset destination
                        for s in styles.iter_mut() {
                            s.set_source_prefix(&assets_href_base);
                        }
                        self.styles_cache.push(dep);
                    }
                    if plugin.scripts.is_some()
                        && !apply.scripts_match.is_empty()
                    {
                        let mut dep = dep.clone();
                        let scripts = dep
                            .plugin
                            .as_mut()
                            .unwrap()
                            .scripts
                            .as_mut()
                            .unwrap();
                        // Make script paths relative to the plugin asset destination
                        for s in scripts.iter_mut() {
                            s.set_source_prefix(&assets_href_base);
                        }
                        self.scripts_cache.push(dep);
                    }

                    // Got some layouts to apply so add to the cache
                    if !apply.layouts_match.is_empty() {
                        let templates =
                            plugin.templates.as_ref().ok_or_else(|| {
                                Error::ApplyLayoutNoTemplate(dep.to_string())
                            })?;
                        let templates =
                            templates.get(engine).ok_or_else(|| {
                                Error::ApplyLayoutNoTemplateForEngine(
                                    dep.to_string(),
                                    engine.to_string(),
                                )
                            })?;
                        let layouts =
                            templates.layouts.as_ref().ok_or_else(|| {
                                Error::ApplyLayoutNoLayouts(
                                    dep.to_string(),
                                    engine.to_string(),
                                )
                            })?;

                        for (key, matches) in apply.layouts_match.iter() {
                            if !layouts.contains_key(key) {
                                return Err(Error::ApplyLayoutNoLayoutForKey(
                                    dep.to_string(),
                                    engine.to_string(),
                                    key.clone(),
                                ));
                            }
                            let fqn = plugin.qualified(key);
                            self.layouts_cache.insert(fqn, matches.clone());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn is_index<P: AsRef<Path>>(file: P) -> bool {
        if let Some(nm) = file.as_ref().file_stem() {
            if nm == INDEX_STEM {
                return true;
            }
        }
        false
    }

    pub fn is_markdown_file(&self, file: &PathBuf) -> bool {
        if let Some(ext) = file.extension() {
            let s = ext.to_string_lossy().into_owned();
            let types = self.settings.types.as_ref().unwrap();
            return types.markdown().contains(&s);
        }
        false
    }

    pub fn is_clean<P: AsRef<Path>>(
        &self,
        file: P,
        types: &RenderTypes,
    ) -> bool {
        let target = file.as_ref().to_path_buf();
        let result = target.clone();
        return self.rewrite_index_file(target, result, types).is_some();
    }

    fn has_parse_file_match<P: AsRef<Path>>(
        &self,
        file: P,
        types: &RenderTypes,
    ) -> bool {
        let path = file.as_ref();
        let mut copy = path.to_path_buf();
        for ext in types.render() {
            copy.set_extension(ext);
            if copy.exists() {
                return true;
            }
        }
        false
    }

    // FIXME: make this private again!
    pub(crate) fn rewrite_index_file<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        file: P,
        result: Q,
        types: &RenderTypes,
    ) -> Option<PathBuf> {
        let clean_target = file.as_ref();
        if !RuntimeOptions::is_index(&clean_target) {
            if let Some(parent) = clean_target.parent() {
                if let Some(stem) = clean_target.file_stem() {
                    let mut target = parent.to_path_buf();
                    target.push(stem);
                    target.push(INDEX_STEM);

                    if !self.has_parse_file_match(&target, types) {
                        let clean_result = result.as_ref().clone();
                        if let Some(parent) = clean_result.parent() {
                            if let Some(stem) = clean_result.file_stem() {
                                let mut res = parent.to_path_buf();
                                res.push(stem);
                                res.push(INDEX_STEM);
                                res.set_extension(HTML);
                                return Some(res);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub fn get_type<P: AsRef<Path>>(&self, p: P) -> FileType {
        let types = &self.settings.types.as_ref().unwrap();
        let file = p.as_ref();
        if let Some(ext) = file.extension() {
            let ext = ext.to_string_lossy().into_owned();
            if types.render().contains(&ext) {
                if types.markdown().contains(&ext) {
                    return FileType::Markdown;
                } else {
                    return FileType::Template;
                }
            }
        }
        FileType::Unknown
    }

    pub fn is_page<P: AsRef<Path>>(&self, p: P) -> bool {
        match self.get_type(p) {
            FileType::Markdown | FileType::Template => true,
            _ => false,
        }
    }

    pub fn relative_to<P: AsRef<Path>>(
        &self,
        file: P,
        base: P,
        target: P,
    ) -> Result<PathBuf> {
        let f = file.as_ref().canonicalize()?;
        let b = base.as_ref().canonicalize()?;
        let t = target.as_ref().to_path_buf();
        Ok(t.join(f.strip_prefix(b)?))
    }

    pub fn get_canonical_url(
        &self,
        config: &Config,
        path: Option<&str>,
    ) -> crate::Result<Url> {
        let mut base = self.settings.get_canonical_url(config)?;
        if let Some(path) = path {
            base = base.join(path)?;
        }
        Ok(base)
    }

    pub fn get_assets_path(&self) -> PathBuf {
        self.source.join(self.settings.assets.as_ref().unwrap())
    }

    pub fn get_includes_path(&self) -> PathBuf {
        self.source.join(self.settings.includes.as_ref().unwrap())
    }

    pub fn get_partials_path(&self) -> PathBuf {
        self.source.join(self.settings.partials.as_ref().unwrap())
    }

    pub fn get_data_sources_path(&self) -> PathBuf {
        self.source
            .join(self.settings.data_sources.as_ref().unwrap())
    }

    pub fn get_locales(&self) -> PathBuf {
        self.source.join(self.settings.locales.as_ref().unwrap())
    }

    pub fn get_render_types(&self) -> &RenderTypes {
        self.settings.types.as_ref().unwrap()
    }

    /// Convert a href path into a PathBuf relative to the source
    /// directory.
    pub fn resolve_source(&self, href: &str) -> PathBuf {
        self.source
            .join(utils::url::to_path_separator(href.trim_start_matches("/")))
    }

    pub fn relative<P: AsRef<Path>, B: AsRef<Path>>(
        &self,
        href: &str,
        path: P,
        base: B,
    ) -> Result<String> {
        let rel = path.as_ref().strip_prefix(base.as_ref())?;

        let types = self.settings.types.as_ref().unwrap();
        let include_index = self.settings.should_include_index();
        let rewrite_index = self.settings.should_rewrite_index();

        let up = "../";
        let mut value: String = "".to_string();
        if let Some(p) = rel.parent() {
            if rewrite_index && self.is_clean(path.as_ref(), types) {
                value.push_str(up);
            }
            for _ in p.components() {
                value.push_str(up);
            }
        }

        value.push_str(&href.trim_start_matches("/"));

        if include_index && (value.ends_with("/") || value == "") {
            value.push_str(super::INDEX_HTML);
        }

        if !rewrite_index && value == "" {
            value = up.to_string();
        }
        Ok(value)
    }

    // Attempt to get an absolute URL path for a page
    // relative to the source. The resulting href
    // can be passed to the link helper to get a
    // relative path.
    pub fn absolute<F: AsRef<Path>>(
        &self,
        file: F,
        options: LinkOptions,
    ) -> Result<String> {
        let src = if let Some(ref source) = options.strip {
            source
        } else {
            &self.source
        };

        let page = file.as_ref();

        /*
        if !page.starts_with(src) {
            return Err(Error::PageOutsideSource(
                page.to_path_buf(),
                src.to_path_buf(),
            ));
        }
        */

        let mut rel = page.strip_prefix(src)?.to_path_buf();

        if is_home_index(&rel) {
            return Ok("/".to_string());
        }

        let rewrite_index = self.settings.should_rewrite_index();
        if options.rewrite && rewrite_index {
            rel.set_extension("");
            if let Some(stem) = rel.file_stem() {
                if options.include_index {
                    if stem == INDEX_STEM {
                        rel.set_extension(crate::HTML);
                    } else {
                        rel.push(crate::INDEX_HTML);
                    }
                } else {
                    if stem == INDEX_STEM {
                        if let Some(parent) = rel.parent() {
                            rel = parent.to_path_buf();
                        }
                    }
                }
            }
        }

        if options.transpose {
            if let Some(ref types) = self.settings.types {
                if let Some(ext) = rel.extension() {
                    let ext = ext.to_string_lossy().into_owned();
                    if let Some(ref map_ext) = types.map().get(&ext) {
                        rel.set_extension(map_ext);
                    }
                }
            }
        }

        to_href(rel, options)
    }

    pub fn destination(&self) -> DestinationBuilder {
        DestinationBuilder::new(self)
    }
}

pub struct LinkOptions {
    // Convert paths to forward slashes
    pub slashes: bool,
    // Use a leading slash
    pub leading: bool,
    // Use a trailing slash
    pub trailing: bool,
    // Transpose the file extension
    pub transpose: bool,
    // Rewrite to index links when rewrite_index
    pub rewrite: bool,
    // Include index.html when rewrite is active
    pub include_index: bool,
    // Strip this prefix
    pub strip: Option<PathBuf>,
}

impl Default for LinkOptions {
    fn default() -> Self {
        Self {
            slashes: true,
            leading: true,
            trailing: true,
            transpose: true,
            rewrite: true,
            include_index: false,
            strip: None,
        }
    }
}

fn is_home_index<P: AsRef<Path>>(p: P) -> bool {
    let rel = p.as_ref();
    if rel.components().count() == 1 {
        if let Some(stem) = rel.file_stem() {
            if stem == INDEX_STEM {
                return true;
            }
        }
    }
    false
}

fn to_href<R: AsRef<Path>>(rel: R, options: LinkOptions) -> Result<String> {
    let rel = rel.as_ref();

    let mut href = if options.leading {
        "/".to_string()
    } else {
        "".to_string()
    };
    let value = if options.slashes {
        utils::url::to_href_separator(&rel)
    } else {
        rel.to_string_lossy().into_owned()
    };

    href.push_str(&value);

    if options.trailing && rel.extension().is_none() {
        href.push('/');
    }
    Ok(href)
}

#[derive(Debug)]
pub struct DestinationBuilder<'a> {
    pub options: &'a RuntimeOptions,
    // Request a 1:1 output file
    pub exact: bool,
    // Rewrite to directory index.html file
    pub rewrite_index: bool,
    // A base href used to extract sub-directories
    pub base_href: &'a Option<String>,
}

impl<'a> DestinationBuilder<'a> {
    pub fn new(options: &'a RuntimeOptions) -> Self {
        Self {
            options,
            exact: false,
            rewrite_index: options.settings.should_rewrite_index(),
            base_href: &options.settings.base_href,
        }
    }

    pub fn exact(mut self, exact: bool) -> Self {
        self.exact = exact;
        self
    }

    pub fn rewrite_index(mut self, rewrite_index: bool) -> Self {
        self.rewrite_index = rewrite_index;
        self
    }

    pub fn base_href(mut self, base_href: &'a Option<String>) -> Self {
        self.base_href = base_href;
        self
    }

    // Build the output file path.
    //
    // Does not modify the file extension, rewrite the index of change the slug,
    // this is used when we copy over files with a direct 1:1 correlation.
    //
    fn output(&self, pth: &PathBuf) -> Result<PathBuf> {
        //let pth = self.file.clone();

        // NOTE: When watching files we can get absolute
        // NOTE: paths passed for `file` even when `source`
        // NOTE: is relative. This handles that case by making
        // NOTE: the `source` absolute based on the current working
        // NOTE: directory.
        let mut src: PathBuf = self.options.source.clone();
        if pth.is_absolute() && src.is_relative() {
            if let Ok(cwd) = std::env::current_dir() {
                src = cwd.clone();
                src.push(&self.options.source);
            }
        }

        let mut relative = pth.strip_prefix(src)?;
        if let Some(ref base) = self.base_href {
            if relative.starts_with(base) {
                relative = relative.strip_prefix(base)?;
            }
        }

        //let result = self.target.clone().join(relative);
        return Ok(relative.to_path_buf());
    }

    // Build the destination file path and update the file extension.
    pub fn build(&mut self, pth: &PathBuf) -> Result<PathBuf> {
        let mut result = self.output(pth)?;
        if !self.exact {
            let file_type = self.options.get_type(pth);
            match file_type {
                FileType::Markdown | FileType::Template => {
                    let settings = &self.options.settings;
                    let types = settings.types.as_ref().unwrap();

                    if let Some(ext) = pth.extension() {
                        let ext = ext.to_string_lossy().into_owned();
                        for (k, v) in types.map() {
                            if ext == *k {
                                result.set_extension(v);
                                break;
                            }
                        }
                    }

                    if self.rewrite_index {
                        if let Some(res) =
                            self.options.rewrite_index_file(pth, &result, types)
                        {
                            result = res;
                        }
                    }
                }
                _ => {}
            }
        }
        return Ok(result);
    }
}

#[cfg(test)]
mod tests {
    use crate::link::*;
    use crate::{Config, RuntimeOptions};
    use std::path::PathBuf;

    #[test]
    fn outside_source() -> Result<()> {
        let mut opts: RuntimeOptions = Default::default();
        let source = PathBuf::from("site");
        opts.source = source.clone();
        let page = PathBuf::from("post/article.md");
        let result = opts.absolute(&page, Default::default());
        // TODO: restore this - requires PartialEq on Error
        //assert_eq!(Some(Error::PageOutsideSource(page, source)), result.err());
        Ok(())
    }

    #[test]
    fn absolute_page_extension_rewrite() -> Result<()> {
        let mut opts: RuntimeOptions = Default::default();
        opts.source = PathBuf::from("site");
        let page = PathBuf::from("site/post/article.md");
        let result = opts.absolute(&page, Default::default())?;
        assert_eq!("/post/article.html", result);
        Ok(())
    }

    #[test]
    fn absolute_page() -> Result<()> {
        let mut opts: RuntimeOptions = Default::default();
        opts.source = PathBuf::from("site");
        let page = PathBuf::from("site/post/article.html");
        let result = opts.absolute(&page, Default::default())?;
        assert_eq!("/post/article.html", result);
        Ok(())
    }

    #[test]
    fn absolute_rewrite() -> Result<()> {
        let mut opts: RuntimeOptions = Default::default();
        opts.source = PathBuf::from("site");
        opts.settings.rewrite_index = Some(true);

        let page = PathBuf::from("site/post/article.html");
        let result = opts.absolute(&page, Default::default())?;
        assert_eq!("/post/article/", result);
        Ok(())
    }
}
