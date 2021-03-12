enum State {
    None,
    Inside,
    Between,
}

// Minify an HTML string.
//
// Designed so it doesn't break your markup (not overly agressive)
// and so it does not require parsing a DOM tree (fast).
//
// Finds consecutive sections of whitespace between nodes and ignores
// them.
pub fn html<S: AsRef<str>>(content: S) -> String {
    let s = content.as_ref();
    let mut buf = "".to_string();
    let mut tmp = "".to_string();
    let mut state = State::None;
    let mut empty = true;

    for c in s.chars() {
        if c == '<' {
            if !empty {
                buf.push_str(&tmp);
            }

            tmp = "".to_string();
            state = State::Inside;
        } else if c == '>' {
            if let State::Inside = state {
                state = State::Between;
                empty = true;
                buf.push(c);
                continue;
            }
        }

        match state {
            State::None | State::Inside => {
                buf.push(c);
            }
            _ => {
                empty = empty && c.is_whitespace();
                tmp.push(c);
            }
        }
    }

    if !empty {
        buf.push_str(&tmp);
    }
    buf
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn plain_text() {
        let val = "This is some plain text";
        let res = html(val);
        assert_eq!(res, val.to_string());
    }

    #[test]
    fn doctype() {
        let val = "<!doctype html>";
        let res = html(val);
        assert_eq!(res, val.to_string());
    }

    #[test]
    fn text() {
        let val = "<!doctype html> This is some text";
        let res = html(val);
        assert_eq!(res, val.to_string());
    }

    #[test]
    fn strip() {
        let val = "<p>   <b>bold</b>    <i>italic</i></p>";
        let expect = "<p><b>bold</b><i>italic</i></p>";
        let res = html(val);
        assert_eq!(res, expect.to_string());
    }

    #[test]
    fn strip_inline_text() {
        let val =
            "<p>   <b>bold</b> with some inline text <i>italic</i>   \n</p>";
        let expect = "<p><b>bold</b> with some inline text <i>italic</i></p>";
        let res = html(val);
        assert_eq!(res, expect.to_string());
    }

    #[test]
    fn strip_ignore_script() {
        let val = r#"<script>
    const el = document.querySelector('main > header > .title');
</script>"#;
        let res = html(val);
        assert_eq!(res, val.to_string());
    }

    #[test]
    fn strip_script_comparison() {
        let val = r#"<script>
if (1 < 10 && 12 > 1) {
    if (foo < bar && bar > baz) {

    }
}
</script>"#;
        let res = html(val);
        assert_eq!(res, val.to_string());
    }
}
