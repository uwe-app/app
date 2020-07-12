use config::Page;

use config::RuntimeOptions;

pub fn is_draft(data: &Page, opts: &RuntimeOptions) -> bool {
    if opts.settings.is_release() {
        return data.draft.is_some() && data.draft.unwrap();
    }
    false
}
