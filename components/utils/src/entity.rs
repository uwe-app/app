pub fn unescape(txt: &str) -> String {
    txt.replace("&gt;", ">")
        .replace("&lt;", "<")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

pub fn escape(txt: &str) -> String {
    txt.replace(">", "&gt;")
        .replace("<", "&lt;")
        .replace("&", "&amp;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}
