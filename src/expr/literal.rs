use super::*;

macro_rules! literals {
    ($($name:ident: $ty:ty),*$(,)?) => {
        #[derive(Debug, Clone, PartialEq)]
        pub enum Literal {
            $($name($ty)),*
        }

        $(
            impl From<$ty> for Literal {
                #[inline]
                fn from(v: $ty) -> Literal {
                    Literal::$name(v)
                }
            }
        )*
    }
}

literals! {
    Char: i8,
    Int2: i16,
    Int4: i32,
    Int8: i64,
    Float4: f32,
    Float8: f64,
    TextStr: &'static str,
    TextString: String,
}

impl Expr for Literal {}
impl Collectable for Literal {
    fn collect(&self, w: &mut dyn Write, _: &mut Collector) -> fmt::Result {
        match *self {
            Literal::Char(v) => write!(w, "{}", v),
            Literal::Int2(v) => write!(w, "{}", v),
            Literal::Int4(v) => write!(w, "{}", v),
            Literal::Int8(v) => write!(w, "{}", v),
            Literal::Float4(v) => write!(w, "{}", v),
            Literal::Float8(v) => write!(w, "{}", v),
            Literal::TextStr(v) => write_escaped_string_quoted(v, w),
            Literal::TextString(ref v) => write_escaped_string_quoted(&v, w),
        }
    }
}

fn write_escaped_string_quoted(string: &str, w: &mut dyn Write) -> fmt::Result {
    let escaped = string
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("'", "\\'")
        .replace("\0", "\\0")
        .replace("\x08", "\\b")
        .replace("\x09", "\\t")
        .replace("\x1a", "\\z")
        .replace("\n", "\\n")
        .replace("\r", "\\r");

    w.write_str(if escaped.find('\\').is_some() { "E'" } else { "'" })?;
    w.write_str(&escaped)?;
    w.write_char('\'')
}
