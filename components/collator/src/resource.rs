use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum ResourceKind {
    /// A directory encountered whilst walking the tree.
    Dir,
    /// The default type indicates we don't know much about this resource.
    File,
    /// The type of file that renders to an output page.
    Page,
    /// An asset file is typically located in the `assets` folder and
    /// is primarily used for the site layout: images, fonts, styles etc.
    Asset,
    /// A locale resource, typically .ftl files in the `locales` folder.
    Locale,
    /// A partial file provides part of a template render; normally
    /// located in the `partials` directory but can also come from
    /// other locations.
    Partial,
    /// Include files are documents included by pages; they normally
    /// reside in the `includes` directory and are typically used for
    /// code samples etc.
    Include,
    /// This file is part of a data source directory.
    DataSource,
}

impl Default for ResourceKind {
    fn default() -> Self {
        ResourceKind::File
    }
}

/// The compiler uses this as the action to perform
/// with the input source file.
#[derive(Debug, Clone)]
pub enum ResourceOperation {
    // Do nothing, used for the Dir kind primarily.
    Noop,
    // Render a file as a page template
    Render,
    // Copy a file to the build target
    Copy,
    // Create a symbolic link to the source file
    Link,
}

impl Default for ResourceOperation {
    fn default() -> Self {
        ResourceOperation::Copy
    }
}

#[derive(Debug, Default, Clone)]
pub struct ResourceTarget {
    pub destination: PathBuf,
    pub operation: ResourceOperation,
    pub kind: ResourceKind,
}

impl ResourceTarget {
    pub fn get_output(&self, base: &PathBuf) -> PathBuf {
        base.join(&self.destination)
    }
}

#[derive(Debug, Clone)]
pub enum Resource {
    Page { target: ResourceTarget },
    File { target: ResourceTarget },
}

impl Resource {
    pub fn new(
        destination: PathBuf,
        kind: ResourceKind,
        op: ResourceOperation,
    ) -> Self {
        let target = ResourceTarget {
            kind,
            destination,
            operation: op,
        };
        Resource::File { target }
    }

    pub fn new_page(destination: PathBuf) -> Self {
        let kind = ResourceKind::Page;
        let target = ResourceTarget {
            kind,
            destination,
            operation: ResourceOperation::Render,
        };
        Resource::Page { target }
    }

    pub fn set_operation(&mut self, operation: ResourceOperation) {
        match self {
            Self::Page { ref mut target } | Self::File { ref mut target } => {
                target.operation = operation;
            }
        }
    }
}
