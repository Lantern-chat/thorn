#[derive(Debug, thiserror::Error)]
pub enum SqlFormatError {
    #[error(transparent)]
    FmtError(#[from] std::fmt::Error),

    #[error("Invalid parameter index {0}")]
    InvalidParameterIndex(usize),

    #[error("Confliction parameter type at index {0}: {1} != {2}")]
    ConflictingParameterType(usize, pg::Type, pg::Type),
}

use smallvec::SmallVec;
use std::marker::PhantomData;

pub struct Query<'a, E: From<pgt::Row>> {
    pub q: String,
    pub params: SmallVec<[&'a (dyn pg::ToSql + Sync + 'a); 16]>,
    pub param_tys: SmallVec<[pg::Type; 16]>,
    e: PhantomData<E>,
}

impl<E: From<pgt::Row>> Default for Query<'_, E> {
    fn default() -> Self {
        Query {
            q: String::with_capacity(128),
            params: Default::default(),
            param_tys: Default::default(),
            e: std::marker::PhantomData,
        }
    }
}

use crate::{
    table::{Column, Table, TableExt},
    Literal,
};
use std::{
    collections::hash_map::{Entry, HashMap},
    fmt::{self, Write},
};

use crate::literal::write_escaped_string_quoted;

#[allow(clippy::single_char_add_str)]
impl<E: From<pgt::Row>> Write for Query<'_, E> {
    #[inline(always)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.q.push_str(s);
        self.q.push_str(" ");
        Ok(())
    }

    #[inline(always)]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.q.push(c);
        self.q.push_str(" ");
        Ok(())
    }

    #[inline(always)]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.q.write_fmt(args)?;
        self.q.push_str(" ");
        Ok(())
    }
}

#[allow(clippy::single_char_add_str)]
impl<'a, E: From<pgt::Row>> Query<'a, E> {
    pub fn inner(&mut self) -> &mut String {
        &mut self.q
    }

    pub fn param(&mut self, value: &'a (dyn pg::ToSql + Sync), ty: pg::Type) -> Result<(), SqlFormatError> {
        //if self.params.
        let idx = if let Some(idx) = self.params.iter().position(|&p| {
            // SAFETY: Worst-case parameter duplication, best-case using codegen-units=1 no issues at all
            #[allow(clippy::vtable_address_comparisons)]
            std::ptr::eq(
                p as *const (dyn pg::ToSql + Sync),
                value as *const (dyn pg::ToSql + Sync),
            )
        }) {
            if ty != pg::Type::ANY {
                let existing_ty = &self.param_tys[idx];
                if *existing_ty == pg::Type::ANY {
                    self.param_tys[idx] = ty;
                } else if *existing_ty != ty {
                    return Err(SqlFormatError::ConflictingParameterType(idx, ty, existing_ty.clone()));
                }
            }

            idx + 1 // 1-indexed
        } else {
            self.params.push(value);
            self.param_tys.push(ty);
            self.params.len() // 1-indexed, take len after push
        };

        self.inner().push_str("$");
        self.write_literal(idx as i64).map_err(From::from)
    }

    #[inline(always)]
    pub fn write_literal<L: Literal>(&mut self, lit: L) -> fmt::Result {
        lit.collect_literal(self.inner(), 0)?;
        self.inner().write_str(" ")
    }

    #[inline(always)]
    pub fn write_column<T: TableExt>(&mut self, col: T, name: &'static str) -> fmt::Result {
        write!(
            self.inner(),
            "\"{}\".\"{}\" ",
            if name == T::TYPENAME_SNAKE { <T as Table>::NAME.name() } else { name },
            <T as Column>::name(&col)
        )
    }

    pub fn write_table<T: Table>(&mut self) -> fmt::Result {
        crate::query::from_item::__write_table::<T>(self)
    }

    pub fn write_column_name<C: Column>(&mut self, col: C) -> fmt::Result {
        write!(self.inner(), "\"{}\" ", col.name())
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
    ($($tt:tt)*) => {{
        #[allow(clippy::redundant_closure_call, unreachable_code)]
        (|| -> Result<_, $crate::macros::SqlFormatError> {
            use std::fmt::Write;
            use $crate::*;

            let mut __thorn_query = $crate::macros::Query::<Columns>::default();
            __isql!([] () f __thorn_query; $($tt)*);
            Ok(__thorn_query)
        }())
    }};
}

include!(concat!(env!("OUT_DIR"), "/sql_macro.rs"));

#[cfg(test)]
mod tests {
    use crate::pg::Type;
    use crate::table::*;

    crate::tables! {
        pub struct TestTable as "renamed" in MySchema {
            SomeCol: Type::INT8,
            SomeCol2: Type::INT8,
        }

        pub struct AnonTable {
            Other: Type::BOOL,
        }
    }

    #[test]
    fn test_sql_macro() {
        let y = 21;
        let k = [String::from("test"); 1];

        let res = sql! {
            use std::borrow::{Cow, Borrow};

            SELECT 1 AS @SomCol
        };

        // random hodgepodge of symbols to test the macro
        let res = sql! {
            WITH AnonTable AS (
                SELECT TestTable.SomeCol::{let ty = Type::BIT_ARRAY; ty} AS AnonTable.Other FROM TestTable
            )
            ----
            for-join{"%"} i in [1, 2, 3] {
                SELECT {i}
            }

            {"test"}(1)

            for v in k {
                SELECT {v}
            }

            {"test"}(1)

            if true {
                SELECT {"true"}
            } else {
                if true {
                    SELECT "false"
                } else {
                    TRUE

                    // triggers compile_fail!
                    //SELECT 1 AS @TestTable.SomeCol
                }
            }

            if let Some(value) = Some("") {
                SELECT {value}
            }

            if true { 1 }

            let value = 1;

            AND  call()

            if let Some(v) = Some(1) {

            }

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
                        SELECT "TWO"
                        }
                    },
                    _ => {},
                }
            }

            SELECT AliasTable.SomeCol

            FROM TestTable AS AliasTable

            ARRAY_AGG()
            -- () && || |
            SELECT SIMILAR TO TestTable.SomeCol
            FROM[#{&"test"}, 30]::_int8 #{&23 => Type::TEXT} ; call_func({y}) "hel'lo"::text[] @{"'"}     { let x = 10; x + y } !! TestTable WHERE < AND NOT = #{&1}

            1 AS @SomeCol,
            TestTable.SomeCol2 AS @_

            SELECT
        }
        .unwrap();

        println!("OUT: {}", res.q);
    }
}
