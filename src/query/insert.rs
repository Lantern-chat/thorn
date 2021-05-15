use super::{with::WithQuery, *};
use crate::*;

#[derive(Default)]
pub struct InsertQuery {
    with: Option<WithQuery>,
    into: Option<Box<dyn FromItem>>,
    values: Option<Vec<Box<dyn ValueExpr>>>,
    returning: Option<Box<dyn Expr>>,
}
