use config::Page;

use super::RuntimeOptions;

pub fn is_draft(data: &Page, opts: &RuntimeOptions) -> bool {
    if opts.release {
        return data.draft.is_some() && data.draft.unwrap();
    }
    false
}
