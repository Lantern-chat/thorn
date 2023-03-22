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
    use crate::{
        table::{Column, Table},
        Literal,
    };

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

        pub fn inner(&mut self) -> &mut W {
            &mut self.inner
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
        pub fn write_literal<L: Literal>(&mut self, lit: L) -> fmt::Result {
            lit.collect_literal(self.inner(), 0)?;
            self.inner.write_str(" ")
        }

        pub fn write_column<T: Table>(&mut self, col: T) -> fmt::Result {
            write!(
                self.inner(),
                "\"{}\".\"{}\" ",
                <T as Table>::NAME.name(),
                <T as Column>::name(&col)
            )
        }

        pub fn write_table<T: Table>(&mut self) -> fmt::Result {
            crate::query::from_item::__write_table::<T>(self)
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
}

/// Generate SQL syntax with an SQL-like macro. To make it work with a regular Rust macro
/// certain syntax had to be changed.
///
/// * For function calls `.func()` is converted to `func()`
///     * Runtime function names can be specified with `.{"whatever fmt::Display value"}()`
/// * `--` is converted to `$$`
/// * `::{let ty = Type::INT8_ARRAY; ty}` with any arbitrary code block can be used for dynamic cast types
/// * All string literals (`"string literal"`) are properly escaped and formatted as `'string literal'`
///     * Other literals such as bools and numbers are also properly formatted
/// * Known PostgreSQL Keywords are allowed through, `sql!(SELECT * FROM TestTable)`
/// * Non-keyword identifiers are treated as [`Table`](crate::Table) types.
/// * `Ident::Ident` is treated as a column, so `TestTable::Col` converts to `"test_table"."col"`
///     * `AS Ident::Ident` is treated specially to remove all but the column name for alises.
/// * Arbitrary expressions are allowed with code-blocks `{let x = 10; x + 21}`, but will be converted to [`Literal`](crate::Literal) values.
///     * To escape this behavior, prefix the code block with `@`, so `@{"something weird"}` is added directly as `something weird`, not a string.
/// * Parametric values can be specified with `#{1}` or `#{2 => Type::INT8}` for accumulating types
/// * For-loops in codegen are supported like `for your_variable in your_data { SELECT {your_variable} }
/// * Conditionals are supported via `if condition { SELECT "true" }`
///     * Also supports an `else { SELECT "false" }` branch
#[macro_export]
macro_rules! sql {
    (@WRITER $out:expr) => { $crate::macros::__private::Writer::new($out) };

    (@ADD $writer:expr; $($tt:tt)*) => {{
        #[allow(clippy::redundant_closure_call, unreachable_code)]
        (|| -> Result<(), $crate::macros::SqlFormatError> {
            use std::fmt::Write;
            __isql!([] $writer; $($tt)*);
            Ok(())
        }())
    }};

    ($out:expr; $($tt:tt)*) => {{
        let mut __thorn_writer = sql!(@WRITER $out);
        sql!(@ADD __thorn_writer; $($tt)*).map(|_| __thorn_writer.params)
    }};

    ($($tt:tt)*) => {{
        let mut __thorn_out = String::new();
        sql!(&mut __thorn_out; $($tt)*).map(|_| __thorn_out)
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
                SELECT TestTable.SomeCol::{let ty = Type::BIT_ARRAY; ty} AS AnonTable.Other FROM TestTable
            )
            ----
            for-join{"%"} i in [1, 2, 3] {
                SELECT #{i}
            }

            .{"test"}(1)

            for v in k {
                SELECT {v}
            }

            .{"test"}(1)

            if true {
                SELECT {"true"}
            } else {
                if true {
                    SELECT "false"
                } else {
                    TRUE
                }
            }

            if let Some(value) = Some("") {
                SELECT {value}
            }

            if true { return; }

            let value = 1;

            AND  .call()

            match value {
                2 => {},
                1 | 3 if true => {
                    SELECT "ONE"
                },
                _ => {},
            }

            for (idx, term) in [1, 2, 3].iter().enumerate() {
                match idx {
                    2 => {},
                    1 | 3 if true => {
                        if false {
                        SELECT "ONE"
                        }
                    },
                    _ => {},
                }
            }

            ARRAY_AGG()
            -- () && || |
            SELECT SIMILAR TO TestTable.SomeCol
            FROM[#{{let x = 23; x}}, 30]::_int8 #{23 => Type::TEXT} ; .call_func({y}) "hel'lo"::text[] @{"'"}  { let x = 10; x + y } !! TestTable WHERE < AND NOT = #{1}

            return;
            // does not even parse after return;
            SELECT test
        }
        .unwrap();

        println!("OUT: {}", res);
    }
}
