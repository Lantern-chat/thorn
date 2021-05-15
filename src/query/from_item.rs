use crate::{
    collect::{Collectable, Collector},
    *,
};

use std::{
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
            Schema::None => write!(w, "\"{}\"", T::NAME.name()),
            Schema::Named(name) => write!(w, "\"{}\".\"{}\"", name, T::NAME.name()),
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
    l: L,
    r: R,
    conds: Vec<Box<dyn BooleanExpr>>,
    kind: JoinType,
}

impl<L, R> Join<L, R> {
    pub fn on<E>(mut self, expr: E) -> Self
    where
        E: BooleanExpr + 'static,
    {
        self.conds.push(Box::new(expr));
        self
    }
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

        let mut conds = self.conds.iter();

        if let Some(cond) = conds.next() {
            w.write_str(" ON ")?;
            let wrap_paren = conds.len() > 1;
            if wrap_paren {
                w.write_str("(")?;
            }
            cond._collect(w, t)?;

            for cond in conds {
                w.write_str(" AND ")?;
                cond._collect(w, t)?;
            }

            if wrap_paren {
                w.write_str(")")?;
            }
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

pub trait TableJoinExt: Table {
    fn join<F: FromItem>(kind: JoinType, with: F) -> Join<TableRef<Self>, F> {
        Join {
            l: TableRef::new(),
            r: with,
            conds: Vec::new(),
            kind,
        }
    }

    fn inner_join<F: FromItem>(with: F) -> Join<TableRef<Self>, F> {
        Self::join(JoinType::InnerJoin, with)
    }

    fn inner_join_table<T: Table>() -> Join<TableRef<Self>, TableRef<T>> {
        Self::inner_join(TableRef::new())
    }

    fn left_join<F: FromItem>(with: F) -> Join<TableRef<Self>, F> {
        Self::join(JoinType::LeftJoin, with)
    }

    fn left_join_table<T: Table>() -> Join<TableRef<Self>, TableRef<T>> {
        Self::left_join(TableRef::new())
    }

    fn right_join<F: FromItem>(with: F) -> Join<TableRef<Self>, F> {
        Self::join(JoinType::RightJoin, with)
    }

    fn right_join_table<T: Table>() -> Join<TableRef<Self>, TableRef<T>> {
        Self::right_join(TableRef::new())
    }

    fn full_outer_join<F: FromItem>(with: F) -> Join<TableRef<Self>, F> {
        Self::join(JoinType::FullOuterJoin, with)
    }

    fn full_outer_join_table<T: Table>() -> Join<TableRef<Self>, TableRef<T>> {
        Self::full_outer_join(TableRef::new())
    }
}

impl<T> TableJoinExt for T where T: Table {}
