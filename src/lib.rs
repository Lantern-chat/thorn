#![allow(unused_imports, clippy::wrong_self_convention)]

pub extern crate postgres_types as pg;

#[doc(hidden)]
pub extern crate paste;

pub mod collect;
pub mod expr;
pub mod query;
pub mod ty;

#[macro_use]
pub mod table;

pub use collect::Collectable;
pub use expr::{Expr, *};
pub use query::{Query, TableAsExt, TableJoinExt, WithableQueryExt};
pub use table::Table;

#[cfg(test)]
mod test {
    use pg::Type;

    use super::*;

    use table::TestTable;

    tables! {
        pub struct Users in MySchema {
            Id: Type::INT8,
            UserName: Type::VARCHAR,
        }
    }

    #[test]
    fn test() {
        tables! {
            struct Temp {
                Id: Type::INT4,
            }
        }

        let s = Query::with()
            .with(Temp::as_query(
                Query::select()
                    .expr(Literal::Int4(1).alias_to(Temp::Id))
                    .not_materialized(),
            ))
            .select()
            .distinct()
            .cols(vec![TestTable::Id, TestTable::UserName])
            .expr(Users::Id.cast(Type::INT8))
            .expr(Builtin::coalesce((TestTable::UserName, Users::UserName)))
            .expr(Builtin::count(Any))
            .expr(
                Var::of(Type::INT4)
                    .neg()
                    .abs()
                    .bit_and(Literal::Int4(63))
                    .cast(Type::BOOL)
                    .is_not_unknown()
                    .rename_as("Test")
                    .unwrap(),
            )
            .from(TestTable::left_join_table::<Users>().on(TestTable::UserName.equals(Users::UserName)))
            .and_where(Users::Id.equals(Var::of(Type::INT8)))
            .and_where(
                Users::UserName
                    .equals(Var::of(Type::TEXT))
                    .or(Users::UserName.like("%Test%")),
            )
            .and_where(Users::Id.less_than(Builtin::OctetLength.arg(Users::Id)))
            .limit_n(10)
            .offset_n(1)
            .order_by(TestTable::Id.ascending().nulls_first())
            .order_by(TestTable::UserName.descending())
            .and_where(Users::UserName.like("%Test%"))
            .and_where(Query::select().expr(Var::of(Type::TEXT)).exists())
            .and_where(
                Query::select()
                    .col(Users::Id)
                    .from_table::<Users>()
                    .any()
                    .less_than(Var::of(Type::INT4)),
            )
            .union_all(
                Query::select()
                    .exprs(std::iter::repeat(Literal::Int4(1)).take(6)) // must match length of other queries
                    .from_table::<Users>(),
            )
            .to_string();

        println!("{}", s.0);
    }
}
