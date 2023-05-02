use std::fmt::{self, Write};

use proc_macro2::Ident;
use syn::{parse::ParseStream, Lit};

pub(crate) fn parse_lit(input: ParseStream, state: &mut super::State) -> syn::Result<()> {
    let start = input.fork();

    match input.parse()? {
        lit @ (Lit::Int(_) | Lit::Float(_)) => state.push(lit),
        Lit::Bool(b) => {
            state.push(Ident::new(if b.value { "TRUE" } else { "FALSE" }, b.span));
        }
        Lit::Str(s) => state.push_str({
            let mut buf = String::new();
            write_escaped_string_quoted(&s.value(), &mut buf).unwrap();
            buf
        }),
        // https://www.postgresql.org/docs/15/datatype-binary.html#id-1.5.7.12.9
        Lit::ByteStr(s) => state.push_str({
            let mut buf = "'\\x".to_owned();
            for byte in s.value() {
                write!(buf, "{byte:0X}").unwrap();
            }
            buf.push_str("'");
            buf
        }),
        _ => return Err(start.error("Literal type is not supported with SQL")),
    }

    Ok(())
}

pub fn escape_string(string: &str) -> String {
    string
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\'', "\\'")
        .replace('\0', "\\0")
        .replace('\x08', "\\b")
        .replace('\x09', "\\t")
        .replace('\x1a', "\\z")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

pub fn write_escaped_string_quoted(string: &str, mut w: impl Write) -> fmt::Result {
    let escaped = escape_string(string);

    w.write_str(if escaped.find('\\').is_some() { "E'" } else { "'" })?;
    w.write_str(&escaped)?;
    w.write_str("'")
}

pub fn write_escaped_string_nested(string: &str, mut w: impl Write) -> fmt::Result {
    let escaped = escape_string(string);

    w.write_str("\"")?;
    w.write_str(&escaped)?;
    w.write_str("\"")
}
