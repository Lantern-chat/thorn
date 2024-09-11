// TODO: Create a new Literal trait with a "collect_as_literal" method, then implement
// this trait AND Collectable/ValueExpr for all literal types (integers, strings, etc.)

use std::{
    borrow::Cow,
    fmt::{self, Write},
};

mod private {
    pub trait Sealed {}

    macro_rules! impl_sealed {
        ($($ty:ty),*) => {$(impl Sealed for $ty {})*}
    }

    impl_sealed!((), bool, i8, i16, i32, i64, f32, f64, &str, String);

    impl<T: Sealed, const N: usize> Sealed for [T; N] {}
    impl<T: Sealed> Sealed for &[T] {}
    impl<T: Sealed> Sealed for Vec<T> {}
    impl<T: Sealed> Sealed for &T {}
}

pub trait Literal: Sized + private::Sealed {
    #[doc(hidden)]
    fn write_literal(&self, w: &mut dyn Write, depth: usize) -> fmt::Result;
}

impl<T: Literal> Literal for &T {
    #[inline]
    fn write_literal(&self, w: &mut dyn Write, depth: usize) -> fmt::Result {
        (**self).write_literal(w, depth)
    }
}

impl Literal for () {
    #[inline]
    fn write_literal(&self, w: &mut dyn Write, _depth: usize) -> fmt::Result {
        w.write_str("NULL")
    }
}

impl Literal for bool {
    #[inline]
    fn write_literal(&self, w: &mut dyn Write, _depth: usize) -> fmt::Result {
        w.write_str(if *self { "TRUE" } else { "FALSE" })
    }
}

macro_rules! impl_num_lits {
    (@INT $($ty:ty),*) => {$(
        impl Literal for $ty {
            fn write_literal(&self, w: &mut dyn Write, _depth: usize) -> fmt::Result {
                w.write_str(itoa::Buffer::new().format(*self))
            }
        }
    )*};

    (@FLOAT $($ty:ty),*) => {$(
        impl Literal for $ty {
            fn write_literal(&self, w: &mut dyn Write, _depth: usize) -> fmt::Result {
                write!(w, "{}", self)
            }
        }
    )*};
}

impl_num_lits!(@INT i8, i16, i32, i64);
impl_num_lits!(@FLOAT f32, f64);

impl Literal for &str {
    fn write_literal(&self, w: &mut dyn Write, depth: usize) -> fmt::Result {
        if depth == 0 {
            write_escaped_string_quoted(self, w)
        } else {
            write_escaped_string_nested(self, w)
        }
    }
}

impl Literal for String {
    fn write_literal(&self, w: &mut dyn Write, depth: usize) -> fmt::Result {
        self.as_str().write_literal(w, depth)
    }
}

impl<T: Literal> Literal for &[T] {
    fn write_literal(&self, mut w: &mut dyn Write, depth: usize) -> fmt::Result {
        if depth == 0 {
            w.write_str("'")?;
        }

        w.write_str("{")?;

        for (i, lit) in self.iter().enumerate() {
            if i > 0 {
                w.write_str(", ")?;
            }

            lit.write_literal(&mut w, depth + 1)?
        }

        w.write_str("}")?;
        if depth == 0 {
            w.write_str("'")?;
        }

        Ok(())
    }
}

impl<T: Literal> Literal for Vec<T> {
    fn write_literal(&self, w: &mut dyn Write, depth: usize) -> fmt::Result {
        self.as_slice().write_literal(w, depth)
    }
}

impl<T: Literal, const N: usize> Literal for [T; N] {
    fn write_literal(&self, w: &mut dyn Write, depth: usize) -> fmt::Result {
        self.as_slice().write_literal(w, depth)
    }
}

macro_rules! impl_literal {
    ($(($($t:ident),*)),*) => {
        $(
            impl<$($t: private::Sealed),*> private::Sealed for ($($t,)*) {}
            impl<$($t: Literal),*> Literal for ($($t,)*) {
                #[allow(non_snake_case)]
                fn write_literal(&self, mut w: &mut dyn Write, depth: usize) -> fmt::Result {
                    if depth == 0 {
                        w.write_str("'")?;
                    }

                    w.write_str("{")?;

                    {
                        let mut __thorn_inc = 0;
                        let ($($t,)*) = self;

                        $(
                            if __thorn_inc > 0 {
                                w.write_str(", ")?;
                            }
                            __thorn_inc += 1;

                            $t.write_literal(&mut w, depth + 1)?;
                        )*
                    }

                    w.write_str("}")?;
                    if depth == 0 {
                        w.write_str("'")?;
                    }

                    Ok(())
                }
            }
        )*
    };
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

#[rustfmt::skip]
fn escape_string(string: &str) -> String {
    let mut out = String::with_capacity(string.len());

    const FIND: &[char] =    &['\\',   '"',    '\'',  '\0',  '\x08', '\x09', '\x1a', '\n',  '\r'];
    const REPLACE: &[&str] = &["\\\\", "\\\"", "\\'", "\\0", "\\b",  "\\t",  "\\z",  "\\n", "\\r"];

    for c in string.chars() {
        if let Some(i) = FIND.iter().position(|&f| f == c) {
            out.push_str(REPLACE[i]);
        } else {
            out.push(c);
        }
    }

    out
}

pub(crate) fn write_escaped_string_quoted(string: &str, mut w: impl Write) -> fmt::Result {
    let escaped = escape_string(string);

    w.write_str(if escaped.find('\\').is_some() { "E'" } else { "'" })?;
    w.write_str(&escaped)?;
    w.write_str("'")
}

fn write_escaped_string_nested(string: &str, mut w: impl Write) -> fmt::Result {
    let escaped = escape_string(string);

    w.write_str("\"")?;
    w.write_str(&escaped)?;
    w.write_str("\"")
}
