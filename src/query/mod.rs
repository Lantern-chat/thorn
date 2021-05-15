pub mod from_item;
pub use from_item::{FromItem, TableJoinExt};

pub mod select;
pub use select::SelectQuery;

pub mod insert;

pub mod call;
pub use call::CallQuery;

use crate::Call;

pub mod with;
pub use with::{TableAsExt, WithQuery, WithableQuery, WithableQueryExt};

pub struct Query;
impl Query {
    pub fn select() -> SelectQuery {
        SelectQuery::default()
    }

    pub fn call(proc: Call) -> CallQuery {
        CallQuery::new(proc)
    }

    pub fn with() -> WithQuery {
        WithQuery::default()
    }
}
