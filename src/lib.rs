#![allow(unused_imports, clippy::wrong_self_convention)]

pub extern crate postgres_types as pg;

pub mod collect;
pub mod expr;
pub mod query;
pub mod ty;

#[macro_use]
pub mod table;

pub use collect::Collectable;
pub use expr::{Expr, *};
pub use query::Query;
pub use table::Table;

#[cfg(test)]
mod test {
    use pg::Type;

    use super::*;

    use table::TestTable;

    table! {
        pub enum Users in MySchema {
            Id: Type::INT8,
            UserName: Type::VARCHAR,
        }
    }

    #[test]
    fn test() {
        let s = Query::select()
            .distinct()
            .from_table::<TestTable>()
            .cols(vec![TestTable::Id, TestTable::UserName])
            .col(Users::Id)
            .expr(TestTable::UserName.coalesce(Users::UserName))
            .expr(
                Var::of(Type::INT4)
                    .neg()
                    .abs()
                    .bit_and(Literal::Int4(63))
                    .cast(Type::BOOL)
                    .is_not_unknown(),
            )
            .join_left_table_on::<Users, _>(TestTable::UserName.equals(Users::UserName))
            .and_where(Users::Id.equals(Var::of(Type::INT8)))
            .and_where(Users::UserName.equals(Var::of(Type::TEXT)))
            .and_where(Users::UserName.like("%Test%"))
            .to_string();

        println!("{}", s.0);
    }
}
