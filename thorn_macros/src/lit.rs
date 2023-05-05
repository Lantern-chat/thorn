use std::fmt::{self, Write};

use proc_macro2::Ident;
use syn::{parse::ParseStream, Error, Lit};

pub fn parse_lit(input: ParseStream) -> syn::Result<Lit> {
    let lit = input.parse()?;
    match lit {
        Lit::Int(_) | Lit::Float(_) | Lit::Str(_) | Lit::ByteStr(_) | Lit::Bool(_) | Lit::Byte(_) => Ok(lit),
        _ => Err(Error::new(lit.span(), "Literal type is not supported with SQL")),
    }
}

pub(crate) fn push_lit(lit: Lit, state: &mut super::State) {
    match lit {
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
        Lit::Byte(b) => state.push_str({
            let mut buf = "x'".to_owned();
            write!(buf, "{:X}'", b.value()).unwrap();
            buf
        }),
        _ => unimplemented!(),
    }
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
