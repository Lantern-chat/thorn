#[doc(hidden)]
pub mod __private {
    #![allow(unused)]

    use std::fmt::{self, Write};

    use crate::literal::write_escaped_string_quoted;

    pub struct Writer<W> {
        inner: W,
        first: bool,
    }

    impl<W: Write> Writer<W> {
        pub fn new(inner: W) -> Self {
            Writer { inner, first: true }
        }

        pub fn write_first(&mut self) -> fmt::Result {
            if self.first {
                self.first = false;
                Ok(())
            } else {
                self.inner.write_str(" ")
            }
        }

        pub fn write_literal<L: Literal>(&mut self, lit: L) -> fmt::Result {
            self.write_first()?;
            lit.write_literal(&mut self.inner)
        }
    }

    impl<W: Write> Write for Writer<W> {
        #[inline(always)]
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.write_first()?;
            self.inner.write_str(s)
        }

        #[inline(always)]
        fn write_char(&mut self, c: char) -> fmt::Result {
            self.write_first()?;
            self.inner.write_char(c)
        }

        #[inline(always)]
        fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
            self.write_first()?;
            self.inner.write_fmt(args)
        }
    }

    pub trait Literal: Sized + fmt::Display {
        #[inline(always)]
        fn write_literal(self, mut out: impl Write) -> fmt::Result {
            write!(out, "{}", self)
        }
    }

    macro_rules! impl_lit {
        ($($ty:ty),*) => {$( impl Literal for $ty {} )*}
    }

    impl_lit!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, isize, usize, f32, f64);

    impl Literal for &str {
        #[inline(always)]
        fn write_literal(self, out: impl Write) -> fmt::Result {
            write_escaped_string_quoted(self, out)
        }
    }

    impl Literal for char {
        #[inline(always)]
        fn write_literal(self, mut out: impl Write) -> fmt::Result {
            out.write_char(self)
        }
    }

    impl Literal for bool {
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
/// * `::{let ty = Type::INT8_ARRAY; ty}` can be used for dynamic cast types
/// * All string literals (`"string literal"`) are properly escaped and formatted as `'string literal'`
/// * Known PostgreSQL Keywords are allowed through, `sql!(SELECT * FROM TestTable)`
/// * Non-keyword identifiers are treated as [`Table`](crate::Table) types.
/// * `Ident::Ident` is treated as a column, so `TestTable::Col` converts to `"test_table"."col"`
///     * Use `@Ident::Ident` to remove the table prefix, useful for `some_value AS @TestTable::Col` aliases, which cannot take the table name
/// * Arbitrary expressions are allowed with code-blocks `{let x = 10; x + 21}`, but will be converted to [`Literal`](crate::Literal) values.
///     * To escape this behavior, prefix the code block with `@`, so `@{"something weird"}` is added directly as `something weird`, not a string.
#[macro_export]
macro_rules! sql {
    ($out:expr; $($tt:tt)*) => {{
        use std::fmt::Write;
        #[allow(clippy::redundant_closure_call)]
        (|| -> std::fmt::Result {
            let mut writer = &mut $crate::macros::__private::Writer::new($out);
            __isql!(&mut writer; $($tt)*);
            Ok(())
        })()
    }};

    ($($tt:tt)*) => {{
        let mut __out = String::new();
        let res = sql!(&mut __out; $($tt)*);
        res.map(|_| __out)
    }};
}

include!(concat!(env!("OUT_DIR"), "/keywords.rs"));

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

        // random hodgepodge of symbols to test the macro
        let res = sql! {
            WITH AnonTable AS (
                SELECT TestTable::SomeCol::{let ty = Type::BIT_ARRAY; ty} AS @AnonTable::Other FROM TestTable
            )
            --
            ARRAY_AGG()
            -- ()
            SELECT SIMILAR TO TestTable::SomeCol
            FROM[#{23}, 30]::_int8 ; .call_func({y}) "hel\"lo"::text[] @{"'"}  { let x = 10; x + y } !! TestTable WHERE < AND NOT = #{1}
        }
        .unwrap();

        println!("OUT: {}", res);
    }
}
