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
            Literal::TextStr(v) => write!(w, "\"{}\"", v),
            Literal::TextString(ref v) => write!(w, "\"{}\"", v),
        }
    }
}
