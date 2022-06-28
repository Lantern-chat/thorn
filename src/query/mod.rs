pub mod from_item;
pub use from_item::{FromItem, Lateral, TableJoinExt};

pub mod select;
pub use select::SelectQuery;

pub mod insert;
pub use insert::InsertQuery;

pub mod call;
pub use call::CallQuery;

pub mod delete;
pub use delete::DeleteQuery;

pub mod update;
pub use update::UpdateQuery;

use crate::{Call, Collectable};

pub mod with;
pub use with::{TableAsExt, WithQuery, WithableQuery, WithableQueryExt};

pub struct Query;
impl Query {
    pub fn select() -> SelectQuery {
        SelectQuery::default()
    }

    pub fn update() -> UpdateQuery<()> {
        UpdateQuery::default()
    }

    pub fn insert() -> InsertQuery<()> {
        InsertQuery::default()
    }

    pub fn delete() -> DeleteQuery<()> {
        DeleteQuery::default()
    }

    pub fn call(proc: Call) -> CallQuery {
        CallQuery::new(proc)
    }

    pub fn with() -> WithQuery {
        WithQuery::default()
    }
}

pub trait AnyQuery: Collectable {}

impl AnyQuery for SelectQuery {}
impl AnyQuery for CallQuery {}
impl<T: crate::Table> AnyQuery for UpdateQuery<T> {}
impl<T: crate::Table> AnyQuery for InsertQuery<T> {}
impl<T: crate::Table> AnyQuery for DeleteQuery<T> {}

#[macro_export]
macro_rules! indexed_columns {
    ($(
        $vis:vis enum $name:ident $(continue $extends:ty)? {
            $first_table:ident::$first:ident
            $(,$table:ident::$col:ident)*
            $(,)?
        }
    )*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        #[repr(usize)]
        $vis enum $name {
            $first = 0 $(+ (<$extends>::offset() as usize))?,

            $($col,)*

            __OFFSET
        }

        impl $name {
            pub const fn offset() -> usize {
                $name::__OFFSET as usize
            }
        }

        impl Default for $name {
            fn default() -> $name {
                $name::__OFFSET
            }
        }

        impl IntoIterator for $name {
            type Item = &'static $first_table;
            type IntoIter = std::slice::Iter<'static, $first_table>;

            fn into_iter(self) -> Self::IntoIter {
                static ITEMS: &[$first_table] = &[
                    $first_table::$first,
                    $($table::$col,)*
                ];

                ITEMS.into_iter()
            }
        }
    )*};
}
