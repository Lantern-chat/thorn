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
    Bool: bool,
    Char: i8,
    Int2: i16,
    Int4: i32,
    Int8: i64,
    Float4: f32,
    Float8: f64,
    TextStr: &'static str,
    TextString: String,
    Array: Vec<Literal>,
}

impl ValueExpr for Literal {}
impl Expr for Literal {}
impl Collectable for Literal {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.collect_nested(w, t, 0)
    }
}

impl Literal {
    fn collect_nested(&self, w: &mut dyn Write, t: &mut Collector, depth: usize) -> fmt::Result {
        match *self {
            Literal::Bool(v) => w.write_str(match v {
                true => "TRUE",
                false => "FALSE",
            }),
            Literal::Char(v) => write!(w, "{}", v),
            Literal::Int2(v) => write!(w, "{}", v),
            Literal::Int4(v) => write!(w, "{}", v),
            Literal::Int8(v) => write!(w, "{}", v),
            Literal::Float4(v) => write!(w, "{}", v),
            Literal::Float8(v) => write!(w, "{}", v),
            Literal::TextStr(v) => write_escaped_string_quoted(v, w),
            Literal::TextString(ref v) => write_escaped_string_quoted(&v, w),
            Literal::Array(ref v) => {
                if depth == 0 {
                    w.write_str("'")?;
                }

                w.write_str("{")?;

                let mut i = 0;

                for lit in v {
                    if i > 0 {
                        w.write_str(", ")?;
                    }

                    match *lit {
                        Literal::TextStr(s) => write_escaped_string_nested(s, w)?,
                        Literal::TextString(ref s) => write_escaped_string_nested(s, w)?,
                        _ => lit.collect_nested(w, t, depth + 1)?,
                    }

                    i += 1;
                }

                w.write_str("}")?;
                if depth == 0 {
                    w.write_str("'")?;
                }

                Ok(())
            }
        }
    }
}

fn escape_string(string: &str) -> String {
    string
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("'", "\\'")
        .replace("\0", "\\0")
        .replace("\x08", "\\b")
        .replace("\x09", "\\t")
        .replace("\x1a", "\\z")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
}

fn write_escaped_string_quoted(string: &str, w: &mut dyn Write) -> fmt::Result {
    let escaped = escape_string(string);

    w.write_str(if escaped.find('\\').is_some() { "E'" } else { "'" })?;
    w.write_str(&escaped)?;
    w.write_str("'")
}

fn write_escaped_string_nested(string: &str, w: &mut dyn Write) -> fmt::Result {
    let escaped = escape_string(string);

    w.write_str("\"")?;
    w.write_str(&escaped)?;
    w.write_str("\"")
}

impl Literal {
    pub const TRUE: Literal = Literal::Bool(true);
    pub const FALSE: Literal = Literal::Bool(false);
    pub const EMPTY_ARRAY: Literal = Literal::Array(Vec::new());
}

impl BooleanExpr for Literal {}

pub trait AsLit {
    fn lit(self) -> Literal;
}

impl AsLit for Literal {
    fn lit(self) -> Literal {
        self
    }
}

macro_rules! impl_literal {
    ($($lit:ty => $which:ident),*) => {
        $(
            impl AsLit for $lit {
                fn lit(self) -> Literal {
                    Literal::$which(self)
                }
            }
        )*
    };

    ($(($($t:ident),*)),*) => {
        $(
            impl<$($t: AsLit),*> AsLit for ($($t,)*) {
                #[allow(non_snake_case)]
                fn lit(self) -> Literal {
                    let ($($t,)*) = self;

                    Literal::Array(vec![
                        $($t.lit()),*
                    ])
                }
            }
        )*
    };
}

impl_literal! {
    bool => Bool,
    i8 => Char,
    i16 => Int2,
    i32 => Int4,
    i64 => Int8,
    f32 => Float4,
    f64 => Float8,
    &'static str => TextStr,
    String => TextString
}

impl<T> AsLit for Vec<T>
where
    T: AsLit,
{
    fn lit(self) -> Literal {
        Literal::Array(self.into_iter().map(AsLit::lit).collect())
    }
}

impl_literal! {
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F),
    (A, B, C, D, E, F, G),
    (A, B, C, D, E, F, G, H),
    (A, B, C, D, E, F, G, H, I),
    (A, B, C, D, E, F, G, H, I, J),
    (A, B, C, D, E, F, G, H, I, J, K),
    (A, B, C, D, E, F, G, H, I, J, K, L),
    (A, B, C, D, E, F, G, H, I, J, K, L, M),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N)
}
