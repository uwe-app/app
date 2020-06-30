use crate::build::page::Page;
use crate::build::CompilerOptions;

pub fn is_draft(data: &Page, opts: &CompilerOptions) -> bool {
    if opts.release {
        return data.draft.is_some() && data.draft.unwrap();
    }
    false
}
