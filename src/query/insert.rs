use crate::{
    collect::{Collectable, Collector},
    order::Order,
    *,
};

use std::fmt::{self, Write};

use super::{from_item::*, with::WithQuery, FromItem};

#[derive(Default)]
pub struct InsertQuery {
    with: Option<WithQuery>,
    into: Option<Box<dyn FromItem>>,
    values: Vec<Box<dyn ValueExpr>>,
    returning: Option<Box<dyn Expr>>,
}

impl InsertQuery {
    fn into<I>(mut self, item: I) -> Self
    where
        I: FromItem + 'static,
    {
        self.into = Some(Box::new(item));
        self
    }

    pub fn into_table<T: Table>(self) -> Self {
        self.into(TableRef::<T>::new())
    }
}

impl Collectable for InsertQuery {
    fn needs_wrapping(&self) -> bool {
        true
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        use crate::expr::util::collect_delimited;

        if let Some(ref with) = self.with {
            with._collect(w, t)?;
            w.write_str(" ")?; // space before INSERT
        }

        w.write_str("INSERT INTO ")?;

        Ok(())
    }
}
