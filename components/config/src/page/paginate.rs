use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PageLink {
    pub index: usize,
    pub name: String,
    pub href: String,
    pub preserve: bool,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaginateInfo {
    // Total number of pages.
    pub total: usize,
    // Current page number.
    pub current: usize,
    // Name of the current page (current + 1)
    pub name: String,
    // Total number of items in the collection.
    pub length: usize,
    // The index into the collection for the
    // first item on this page.
    pub first: usize,
    // The index into the collection for the
    // last item on this page.
    pub last: usize,
    // The actual length of the items in this page,
    // normally the page size but may be less.
    pub size: usize,
    // List of links for each page
    pub links: Vec<PageLink>,
    // Links for next and previous pages when available
    pub prev: Option<PageLink>,
    pub next: Option<PageLink>,
}
