#[derive(Debug, thiserror::Error)]
pub enum SqlFormatError {
    #[error(transparent)]
    FmtError(#[from] std::fmt::Error),

    #[error("Invalid parameter index {0}")]
    InvalidParameterIndex(usize),

    #[error("Confliction parameter type at index {0}: {1} != {2}")]
    ConflictingParameterType(usize, pg::Type, pg::Type),
}

#[doc(hidden)]
pub mod __private {
    #![allow(unused)]

    use super::SqlFormatError;

    use std::{
        collections::btree_map::{BTreeMap, Entry},
        fmt::{self, Write},
    };

    use crate::literal::write_escaped_string_quoted;

    pub struct Writer<W> {
        inner: W,
        pub params: BTreeMap<usize, pg::Type>,
    }

    impl<W: Write> Writer<W> {
        pub fn new(inner: W) -> Self {
            Writer {
                inner,
                params: BTreeMap::default(),
            }
        }

        pub fn param(&mut self, idx: usize, t: pg::Type) -> Result<(), SqlFormatError> {
            if idx < 1 {
                return Err(SqlFormatError::InvalidParameterIndex(idx));
            }

            match self.params.entry(idx) {
                Entry::Occupied(mut t2) if t != *t2.get() => {
                    if *t2.get() == pg::Type::ANY {
                        t2.insert(t);
                    } else if t != pg::Type::ANY {
                        return Err(SqlFormatError::ConflictingParameterType(idx, t, t2.get().clone()));
                    }
                }
                Entry::Vacant(v) => {
                    v.insert(t);
                }
                _ => {}
            }

            Ok(())
        }

        #[inline(always)]
        pub fn write_literal<L: WriteLiteral>(&mut self, lit: L) -> fmt::Result {
            lit.write_literal(&mut self.inner)?;
            self.inner.write_str(" ")
        }
    }

    impl<W: Write> Write for Writer<W> {
        #[inline(always)]
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.inner.write_str(s)?;
            self.inner.write_str(" ")
        }

        #[inline(always)]
        fn write_char(&mut self, c: char) -> fmt::Result {
            self.inner.write_char(c)?;
            self.inner.write_str(" ")
        }

        #[inline(always)]
        fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
            self.inner.write_fmt(args)?;
            self.inner.write_str(" ")
        }
    }

    pub trait WriteLiteral: Sized + fmt::Display {
        #[inline(always)]
        fn write_literal(self, mut out: impl Write) -> fmt::Result {
            write!(out, "{}", self)
        }
    }

    macro_rules! impl_lit {
        ($($ty:ty),*) => {$( impl WriteLiteral for $ty {} )*}
    }

    macro_rules! impl_int_lit {
        ($($ty:ty),*) => {$(
            impl WriteLiteral for $ty {
                #[inline]
                fn write_literal(self, mut out: impl Write) -> fmt::Result {
                    out.write_str(itoa::Buffer::new().format(self))
                }
            }
        )*}
    }

    impl_int_lit!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, isize, usize);
    impl_lit!(f32, f64);

    impl WriteLiteral for &str {
        #[inline(always)]
        fn write_literal(self, out: impl Write) -> fmt::Result {
            write_escaped_string_quoted(self, out)
        }
    }

    impl WriteLiteral for char {
        #[inline(always)]
        fn write_literal(self, mut out: impl Write) -> fmt::Result {
            out.write_char(self)
        }
    }

    impl WriteLiteral for bool {
        #[inline(always)]
        fn write_literal(self, mut out: impl Write) -> fmt::Result {
            out.write_str(if self { "TRUE" } else { "FALSE" })
        }
    }
}

/// Generate SQL syntax with an SQL-like macro. To make it work with a regular Rust macro
/// certain syntax had to be changed.
///
/// * For function calls `.func()` is converted to `func()`
/// * `--` is converted to `$$`
/// * `::{let ty = Type::INT8_ARRAY; ty}` with any arbitrary code block can be used for dynamic cast types
/// * All string literals (`"string literal"`) are properly escaped and formatted as `'string literal'`
/// * Known PostgreSQL Keywords are allowed through, `sql!(SELECT * FROM TestTable)`
/// * Non-keyword identifiers are treated as [`Table`](crate::Table) types.
/// * `Ident::Ident` is treated as a column, so `TestTable::Col` converts to `"test_table"."col"`
///     * `AS Ident::Ident` is treated specially to remove all but the column name for alises.
/// * Arbitrary expressions are allowed with code-blocks `{let x = 10; x + 21}`, but will be converted to [`Literal`](crate::Literal) values.
///     * To escape this behavior, prefix the code block with `@`, so `@{"something weird"}` is added directly as `something weird`, not a string.
/// * Parametric values can be specified with `#{1}` or `#{2 => Type::INT8}` for accumulating types
/// * For-loops in codegen are supported like `for your_variable in your_data; do { SELECT {your_variable} }
/// * Conditionals are supported via `if condition { SELECT "true" }`
///     * Also supports an `else { SELECT "false" }` branch
#[macro_export]
macro_rules! sql {
    (@WRITER $out:expr) => { $crate::macros::__private::Writer::new($out) };

    (@ADD $writer:expr; $($tt:tt)*) => {{
        #[allow(clippy::redundant_closure_call, unreachable_code)]
        (|| -> Result<(), $crate::macros::SqlFormatError> {
            use std::fmt::Write;
            __isql!($writer; $($tt)*);
            Ok(())
        }())
    }};

    ($out:expr; $($tt:tt)*) => {{
        let mut __writer = sql!(@WRITER $out);
        sql!(@ADD __writer; $($tt)*).map(|_| __writer.params)
    }};

    ($($tt:tt)*) => {{
        let mut __out = String::new();
        sql!(&mut __out; $($tt)*).map(|_| __out)
    }};
}

include!(concat!(env!("OUT_DIR"), "/sql_macro.rs"));

#[cfg(test)]
mod tests {
    use crate::pg::Type;
    use crate::table::*;

    crate::tables! {
        pub struct TestTable in MySchema {
            SomeCol: Type::INT8,
        }

        pub struct AnonTable {
            Other: Type::BOOL,
        }
    }

    #[test]
    fn test_sql_macro() {
        let y = 21;
        let k = [String::from("test"); 1];

        // random hodgepodge of symbols to test the macro
        let res = sql! {
            WITH AnonTable AS (
                SELECT TestTable::SomeCol::{let ty = Type::BIT_ARRAY; ty} AS AnonTable::Other FROM TestTable
            )
            ----
            for i in [1, 2, 3]; do {
                SELECT #{i}
            }

            for k in &k; do {
                SELECT {k}
                break;
            }

            if true; do {
                SELECT "true"
            } else {
                SELECT "false"
            }

            if let Some(value) = Some(""); do {
                SELECT {value}
            }

            //if true; do { return; }

            match 1; do {
                2 => {},
                1 | 3 if true => {
                    SELECT "ONE"
                },
                _ => {},
            }

            ARRAY_AGG()
            -- () && || |
            SELECT SIMILAR TO TestTable::SomeCol
            FROM[#{{let x = 23; x}}, 30]::_int8 #{23 => Type::TEXT} ; .call_func({y}) "hel'lo"::text[] @{"'"}  { let x = 10; x + y } !! TestTable WHERE < AND NOT = #{1}

            return;
            // does not even parse after return;
            SELECT test
        }
        .unwrap();

        println!("OUT: {}", res);
    }
}
