use std::fmt::{Write, Error as FmtError};

/// Simple pretty printer hand tailored for the output generated by the `quote`
/// crate for schema inference.
///
/// # Rules
///
/// 1. Seeing `{` increases indentation level
/// 2. Seeing `}` decreases indentation level
/// 3. Insert newline after `{`, `}`, `,`, and `;`
/// 4. Don't put spaces:
///   - between ident and `!`,
///   - between path segments and `::`
///   - after `(`, '<' and before `)`, `>`
///   - before `,`
pub fn format_schema(schema: &str) -> Result<String, FmtError> {
    let mut out = String::with_capacity(schema.len());
    let mut indent = String::new();
    let mut skip_space = false;
    let mut last_char = ' ';
    let mut inside_parenthesis = false;

    for c in schema.chars() {
        // The `quote!` macro inserts whitespaces at some strange location,
        // let's remove them!
        match c {
            '!' | ',' | '<' | ')' | '>' if last_char.is_whitespace() => {
                out.pop();
            }
            ':' if last_char.is_whitespace() => {
                // Unless we are at the beginning of a fully qualified path,
                // remove the whitespace.
                let char_before_whitespace = {
                    let mut chars = out.chars();
                    chars.next_back();
                    chars.next_back()
                };

                if char_before_whitespace != Some('>') {
                    out.pop();
                }
            }
            _ => {}
        }

        if skip_space && c.is_whitespace() && last_char != '>' {
            continue;
        }

        last_char = c;
        skip_space = false;

        // At this point, there is an empty line before `}`. We need to remove
        // the already inserted indent, because the new indent is smaller than
        // the old one.
        if c == '}' {
            while let Some(c) = out.pop() {
                if c == '\n' {
                    break;
                }
            }

            indent.pop();
            write!(out, "\n{}", indent)?;
        }

        // Keep track of our parenthesis level
        match c {
            '(' => inside_parenthesis = true,
            ')' => inside_parenthesis = false,
             _ => {}
        }

        write!(out, "{}", c)?;

        // We need to insert newlines in some places and adjust the indent.
        // Also, we need to remember if we could skip the next whitespace.
        match c {
            ',' => {
                if !inside_parenthesis {
                    skip_space = true;
                    write!(out, "\n{}", indent)?;
                }
            },
            '}' => {
                skip_space = true;
                write!(out, "\n{}", indent)?;
            }
            '{' => {
                skip_space = true;
                indent += "\t";
                write!(out, "\n{}", indent)?;
            }
            ':' | '(' | '<' => skip_space = true,
            _ => {}
        }
    }

    Ok(out.replace("\t", "    ").replace("table!", "\ntable!").trim().to_string())
}


#[cfg(test)]
mod tests {
    use super::format_schema;

    macro_rules! test_pretty_printing {
        ($($name:ident: $input:expr => $expected:expr);*) => {
            $(
                #[test]
                fn $name() {
                    let actual = format_schema($input).unwrap();
                    assert_eq!($expected, actual);
                }
            )*
        }
    }

    test_pretty_printing! {
        test_increase_indent:
            "{,}" =>
            "{\n    ,\n}";

        test_decrease_indent:
            "{abc,}" =>
            "{\n    abc,\n}";

        test_newline_after_comma:
            ",," =>
            ",\n,";

        test_remove_whitespace_macro_call:
            "table ! { }" =>
            "table! {\n}";

        test_remove_whitespace_path_segments:
            ":: diesel :: types :: Text" =>
            "::diesel::types::Text";

        test_remove_whitespace_parenthesis:
            "foo ( 42 )" =>
            "foo (42)";

        test_remove_whitespace_angular_brackets:
            "Option < i32 >" =>
            "Option<i32>";

        test_remove_whitespace_before_comma:
            "id -> Int4 , username -> Varchar" =>
            "id -> Int4,\nusername -> Varchar";

        test_format_nullable:
            "Nullable < :: diesel :: types :: Text >" =>
            "Nullable<::diesel::types::Text>";

        test_format_nullable_multispace:
            "Nullable <  Integer >" =>
            "Nullable<Integer>";

        test_format_arrow:
            "created_at -> :: diesel :: types :: Timestamp" =>
            "created_at -> ::diesel::types::Timestamp";

        test_format_full_line:
            "created_at -> :: diesel :: types :: Timestamp ,," =>
            "created_at -> ::diesel::types::Timestamp,\n,";

        test_format_generated_mod:
            "table ! { users ( id ) { id -> :: diesel :: types :: Int4 , \
            username -> :: diesel :: types :: Varchar , password -> :: diesel :: types :: Varchar \
            , } }" =>
r"table! {
    users (id) {
        id -> ::diesel::types::Int4,
        username -> ::diesel::types::Varchar,
        password -> ::diesel::types::Varchar,
    }
}";
        test_no_newline_after_comma_inside_parenthetis:
            "(a, b)" =>
            "(a, b)"
    }
}
