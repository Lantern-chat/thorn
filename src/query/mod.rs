pub mod from_item;
pub use from_item::FromItem;

pub mod select;
pub use select::SelectQuery;

pub mod call;
pub use call::CallQuery;

use crate::Call;

//pub mod with;
//pub use with::WithableQuery;

pub struct Query;
impl Query {
    pub fn select() -> SelectQuery {
        SelectQuery::default()
    }

    pub fn call(proc: Call) -> CallQuery {
        CallQuery::new(proc)
    }
}
