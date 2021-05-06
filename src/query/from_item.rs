use crate::{
    collect::{Collectable, Collector},
    *,
};

use std::{
    any::Any,
    fmt::{self, Write},
    marker::PhantomData,
    ops::Deref,
};

const _: Option<&dyn FromItem> = None;

pub trait FromItem: Collectable {}

pub struct TableRef<T>(PhantomData<T>);

impl<T> TableRef<T> {
    pub const fn new() -> Self {
        TableRef(PhantomData)
    }
}

impl<T: Table> FromItem for TableRef<T> {}
impl<T: Table> Collectable for TableRef<T> {
    fn collect(&self, w: &mut dyn Write, _: &mut Collector) -> fmt::Result {
        use crate::table::Schema;

        match T::SCHEMA {
            Schema::None => write!(w, "\"{}\"", T::NAME),
            Schema::Named(name) => write!(w, "\"{}\".\"{}\"", name, T::NAME),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoinType {
    InnerJoin,
    LeftJoin,
    RightJoin,
    FullOuterJoin,
}

pub struct Join<L, R> {
    pub l: L,
    pub r: R,
    pub cond: Option<Box<dyn Expr>>,
    pub kind: JoinType,
}

impl<L: FromItem, R: FromItem> FromItem for Join<L, R> {}
impl<L: FromItem, R: FromItem> Collectable for Join<L, R> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.l.collect(w, t)?;
        w.write_str(match self.kind {
            JoinType::InnerJoin => " INNER JOIN ",
            JoinType::LeftJoin => " LEFT JOIN ",
            JoinType::RightJoin => " RIGHT JOIN ",
            JoinType::FullOuterJoin => "FULL OUTER JOIN ",
        })?;
        self.r.collect(w, t)?;

        if let Some(ref cond) = self.cond {
            w.write_str(" ON ")?;
            cond._collect(w, t)?;
        }

        Ok(())
    }
}

impl<T> FromItem for T
where
    T: Deref,
    <T as Deref>::Target: FromItem,
{
}
