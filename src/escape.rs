pub fn string_to_escape_to_c_ansi_id(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' => result.push(c),
            _ => {
                result.push_str(&format!("_X{:X}_", c as u32));
            }
        }
    }
    result
}

pub fn string_from_escape_to_c_ansi_id(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '_' {
            if let Some(c) = chars.next() {
                if c == 'X' {
                    let mut hex = String::new();
                    for c in chars.by_ref() {
                        if c == '_' {
                            break;
                        }
                        hex.push(c);
                    }
                    if let Some(c) = std::char::from_u32(u32::from_str_radix(&hex, 16).unwrap()) {
                        result.push(c);
                    } else {
                        result.push('_');
                        result.push('X');
                        result.push_str(&hex);
                        result.push('_');
                    }
                } else {
                    result.push('_');
                    result.push(c);
                }
            } else {
                result.push('_');
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn format_to_escape_replace(mut code: String) -> String {
    let regex = regex::Regex::new(r"(\{([^\}]+?)\})").unwrap();
    while let Some(captures) = regex.captures(&code) {
        let variable = captures.get(2).unwrap().as_str();
        let variable = string_to_escape_to_c_ansi_id(variable);
        code = code.replace(captures.get(1).unwrap().as_str(), &variable);
    }
    code
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_string_escape_to_c_ansi_id() {
        assert_eq!(string_to_escape_to_c_ansi_id("abc"), "abc");
        assert_eq!(string_to_escape_to_c_ansi_id("ab_c"), "ab_X5F_c");
        assert_eq!(string_to_escape_to_c_ansi_id("a b c"), "a_X20_b_X20_c");
        assert_eq!(
            string_to_escape_to_c_ansi_id("a b c!"),
            "a_X20_b_X20_c_X21_"
        );
        assert_eq!(string_to_escape_to_c_ansi_id("🈶🍤"), "_X1F236__X1F364_");
        assert_eq!(
            string_to_escape_to_c_ansi_id("#:lam1-x"),
            "_X23__X003A_lam1_X2D_x"
        );
        assert_eq!(
            string_to_escape_to_c_ansi_id("#:lam1-y"),
            "_X23__X003A_lam1_X2D_y"
        );
    }

    #[test]
    fn test_string_from_escape_to_c_ansi_id() {
        assert_eq!(string_from_escape_to_c_ansi_id("abc"), "abc");
        assert_eq!(string_from_escape_to_c_ansi_id("ab_X5F_c"), "ab_c");
        assert_eq!(string_from_escape_to_c_ansi_id("a_X20_b_X20_c"), "a b c");
        assert_eq!(
            string_from_escape_to_c_ansi_id("a_X20_b_X20_c_X21_"),
            "a b c!"
        );
        assert_eq!(string_from_escape_to_c_ansi_id("_X1F236__X1F364_"), "🈶🍤");
        assert_eq!(
            string_from_escape_to_c_ansi_id("_X23__X003A_lam1_X2D_x"),
            "#:lam1-x"
        );
        assert_eq!(
            string_from_escape_to_c_ansi_id("_X23__X003A_lam1_X2D_y"),
            "#:lam1-y"
        );
    }

    #[test]
    fn test_format_to_escape_replace() {
        assert_eq!(
            format_to_escape_replace("Hello, {_a_b_C}!".to_string()),
            "Hello, _X5F_a_X5F_b_X5F_C!"
        );
    }
}
