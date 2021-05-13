use super::*;
use crate::*;

enum Values {
    Default,
    Values(Vec<Box<dyn ValueExpr>>),
}

pub struct InsertQuery {
    into: Option<Box<dyn FromItem>>,
    values: Values,
    returning: Option<Box<dyn Expr>>,
}
