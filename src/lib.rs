#![allow(unused_imports, clippy::wrong_self_convention)]

pub extern crate postgres_types as pg;

#[doc(hidden)]
pub extern crate paste;

pub mod collect;
pub mod expr;
pub mod name;
pub mod query;
pub mod ty;

#[macro_use]
pub mod table;

#[macro_use]
pub mod enums;

pub use collect::Collectable;
pub use expr::{Expr, *};
pub use query::{AnyQuery, Lateral, Query, TableAsExt, TableJoinExt, WithableQueryExt};
pub use table::Table;

#[cfg(test)]
mod test {
    use pg::Type;

    use super::*;

    use enums::TestEnum;
    use table::TestTable;

    tables! {
        pub struct Users in MySchema {
            Id: Type::INT8,
            UserName: Type::VARCHAR,
        }

        pub struct Messages in MySchema {
            Id: Type::INT8,
            Author: Type::INT8,
            Content: Type::TEXT,
        }
    }

    indexed_columns! {
        pub enum TestColumns {
            Messages::Id,
        }

        pub enum TestColumns2 {
            Users::Id = TestColumns::offset(),
        }
    }

    #[test]
    fn test_update() {
        tables! {
            struct Temp {
                _Id: Type::INT4,
            }
        }

        let s = Query::with()
            .with(Temp::as_query(
                Query::select()
                    .expr(Literal::Int4(1).alias_to(Temp::_Id))
                    .not_materialized(),
            ))
            .update()
            .only()
            .table::<Users>()
            .set(Users::Id, Temp::_Id)
            .and_where(Users::UserName.equals(Var::of(Users::UserName)))
            .returning(Users::Id)
            .to_string();

        println!("{}", s.0);
    }

    #[test]
    fn test_delete() {
        let s = Query::delete()
            .from::<Users>()
            .only()
            .and_where(Users::UserName.equals(Var::of(Users::UserName)))
            .returning(Users::Id.rename_as("user_id").expect("Invalid name"))
            .to_string();

        println!("{}", s.0);
    }

    #[test]
    fn test_insert() {
        let s = || {
            Query::insert()
                .into::<Users>()
                .cols(Users::COLUMNS)
                .values(vec![Var::of(Users::Id), Var::of(Users::UserName)])
                // or .cols(&[Users::Id, Users::UserName])
                .returning(Users::Id)
                .on_conflict(
                    [Users::Id],
                    DoUpdate.set(Users::UserName, Literal::TextStr("test")),
                )
        };

        let s = s().to_string();

        println!("{}", s.0);
    }

    #[test]
    fn test_lateral() {
        tables! {
            struct Temp {
                Id: Users::Id,
            }
        }

        let s = Query::select()
            .col(Temp::Id)
            .from(
                Users::inner_join_table::<Messages>()
                    .on(Messages::Author.equals(Users::Id))
                    .left_join_table::<Users>()
                    .on(Literal::TRUE)
                    .left_join(Lateral(Temp::as_query(
                        Query::select()
                            .expr(Users::Id.alias_to(Temp::Id))
                            .from_table::<Users>(),
                    )))
                    .on(Literal::TRUE),
            )
            .to_string();

        println!("{}", s.0);
    }

    #[test]
    fn test_select() {
        tables! {
            struct Temp {
                _Id: Users::Id,
            }

            struct Temp2 {
                _Id: Users::Id,
                RowNumber: Type::INT8,
            }
        }

        let s = Query::with()
            .with(Temp::as_query(
                Query::select()
                    .expr(Literal::Int4(1).alias_to(Temp::_Id))
                    .expr(Case::default().when_condition(Temp::_Id.is_not_null(), Literal::Int4(1)))
                    .expr(If::condition(Temp::_Id.is_not_null()).then(Literal::Int4(2)))
                    .not_materialized(),
            ))
            .with(Temp2::as_query(
                Query::select().expr(Users::Id.alias_to(Temp2::_Id)).expr(
                    Builtin::row_number(())
                        .over(Users::Id.ascending())
                        .alias_to(Temp2::RowNumber),
                ),
            ))
            .select()
            .distinct()
            .col(Temp::_Id)
            .cols(&[TestTable::Id, TestTable::UserName])
            .expr(Users::Id.cast(Type::INT8))
            .expr(Builtin::coalesce((TestTable::UserName, Users::UserName)))
            .expr(Builtin::count(Any))
            .expr(TestEnum::Test)
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
                    .equals(Var::of(Users::UserName))
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
                    .exprs(std::iter::repeat(Literal::Int4(1)).take(8)) // must match length of other queries
                    .from_table::<Users>(),
            )
            .group_by(Users::Id)
            .to_string();

        println!("{}", s.0);
    }

    #[test]
    fn test_array_nonnull() {
        let s = Query::select()
            .from_table::<Users>()
            .expr(Builtin::array_agg_nonnull(Users::Id))
            .expr((1, 2, vec!["test"]).lit())
            .to_string();

        println!("{}", s.0);
    }
}
