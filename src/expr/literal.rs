use super::*;

// TODO: Create a new Literal trait with a "collect_as_literal" method, then implement
// this trait AND Collectable/ValueExpr for all literal types (integers, strings, etc.)

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
    fn collect_literal(&self, w: &mut dyn Write, t: &mut Collector, depth: usize) -> fmt::Result;

    fn lit(self) -> Lit<Self> {
        Lit(self)
    }
}

impl<T: Literal> Literal for &T {
    fn collect_literal(&self, w: &mut dyn Write, t: &mut Collector, depth: usize) -> fmt::Result {
        (**self).collect_literal(w, t, depth)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Lit<L: Literal>(pub L);

impl<L: Literal> Expr for Lit<L> {}
impl<L: Literal> ValueExpr for Lit<L> {}
impl<L: Literal> Collectable for Lit<L> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.0.collect_literal(w, t, 0)
    }
}

impl Literal for () {
    fn collect_literal(&self, w: &mut dyn Write, _t: &mut Collector, _depth: usize) -> fmt::Result {
        w.write_str("NULL")
    }
}

impl BooleanExpr for Lit<bool> {}
impl Literal for bool {
    fn collect_literal(&self, w: &mut dyn Write, _t: &mut Collector, _depth: usize) -> fmt::Result {
        w.write_str(if *self { "TRUE" } else { "FALSE" })
    }
}

macro_rules! impl_num_lits {
    (@INT $($ty:ty),*) => {$(
        impl Literal for $ty {
            fn collect_literal(&self, w: &mut dyn Write, _t: &mut Collector, _depth: usize) -> fmt::Result {
                w.write_str(itoa::Buffer::new().format(*self))
            }
        }
    )*};

    (@FLOAT $($ty:ty),*) => {$(
        impl Literal for $ty {
            fn collect_literal(&self, w: &mut dyn Write, _t: &mut Collector, _depth: usize) -> fmt::Result {
                write!(w, "{}", *self)
            }
        }
    )*};
}

impl_num_lits!(@INT i8, i16, i32, i64);
impl_num_lits!(@FLOAT f32, f64);

impl Literal for &str {
    fn collect_literal(&self, w: &mut dyn Write, _t: &mut Collector, depth: usize) -> fmt::Result {
        if depth == 0 {
            write_escaped_string_quoted(self, w)
        } else {
            write_escaped_string_nested(self, w)
        }
    }
}

impl Literal for String {
    fn collect_literal(&self, w: &mut dyn Write, t: &mut Collector, depth: usize) -> fmt::Result {
        self.as_str().collect_literal(w, t, depth)
    }
}

impl<T: Literal> Literal for &[T] {
    fn collect_literal(&self, mut w: &mut dyn Write, t: &mut Collector, depth: usize) -> fmt::Result {
        if depth == 0 {
            w.write_str("'")?;
        }

        w.write_str("{")?;

        for (i, lit) in self.iter().enumerate() {
            if i > 0 {
                w.write_str(", ")?;
            }

            lit.collect_literal(&mut w, t, depth + 1)?
        }

        w.write_str("}")?;
        if depth == 0 {
            w.write_str("'")?;
        }

        Ok(())
    }
}

impl<T: Literal> Literal for Vec<T> {
    fn collect_literal(&self, w: &mut dyn Write, t: &mut Collector, depth: usize) -> fmt::Result {
        self.as_slice().collect_literal(w, t, depth)
    }
}

impl<T: Literal, const N: usize> Literal for [T; N] {
    fn collect_literal(&self, w: &mut dyn Write, t: &mut Collector, depth: usize) -> fmt::Result {
        self.as_slice().collect_literal(w, t, depth)
    }
}

use std::fmt;
impl<T: Literal> fmt::Display for Lit<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut c = Collector::default();
        self.0.collect_literal(f, &mut c, 0)
    }
}

macro_rules! impl_lit_binary_ops {
    ($expr:ident => $($op_trait:ident::$op:ident),*) => {$(
        impl<T: Literal, E> std::ops::$op_trait<E> for Lit<T> {
            type Output = $expr<Self, E>;

            fn $op(self, rhs: E) -> Self::Output {
                <Self as BinaryExt>::$op(self, rhs)
            }
        }
    )*};
}

impl_lit_binary_ops!(BinaryExpr => Add::add, Sub::sub, Mul::mul, Div::div, Rem::rem, BitAnd::bitand, BitOr::bitor, BitXor::bitxor);

macro_rules! impl_literal {
    ($(($($t:ident),*)),*) => {
        $(
            impl<$($t: private::Sealed),*> private::Sealed for ($($t,)*) {}
            impl<$($t: Literal),*> Literal for ($($t,)*) {
                #[allow(non_snake_case)]
                fn collect_literal(&self, mut w: &mut dyn Write, t: &mut Collector, depth: usize) -> fmt::Result {
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

                            $t.collect_literal(&mut w, t, depth + 1)?;
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

fn escape_string(string: &str) -> String {
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
