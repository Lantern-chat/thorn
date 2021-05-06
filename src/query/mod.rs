pub mod from_item;
pub use from_item::FromItem;

pub mod select;
pub use select::SelectQuery;

pub struct Query;

impl Query {
    pub fn select() -> SelectQuery {
        SelectQuery::default()
    }
}
