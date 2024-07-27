use crate::{
    collect::{Collectable, Collector},
    *,
};

use std::{
    fmt::{self, Write},
    marker::PhantomData,
    ops::Deref,
};

use super::{with::NamedQuery, SelectQuery, WithableQuery};

const _: Option<&dyn FromItem> = None;

pub trait FromItem: Collectable {}

#[derive(Default)]
pub struct TableRef<T>(PhantomData<T>);

impl<T> TableRef<T> {
    pub const fn new() -> Self {
        TableRef(PhantomData)
    }
}

#[doc(hidden)]
pub fn __write_table<T: Table>(mut w: impl Write) -> fmt::Result {
    use crate::name::Schema;

    match (T::ALIAS, T::SCHEMA) {
        (None, Schema::None) => write!(w, "\"{}\"", T::NAME.name()),
        (None, Schema::Named(name)) => write!(w, "\"{}\".\"{}\"", name, T::NAME.name()),
        (Some(alias), Schema::None) => write!(w, "\"{}\" AS {alias}", T::NAME.name()),
        (Some(alias), Schema::Named(name)) => write!(w, "\"{}\".\"{}\" AS {alias}", name, T::NAME.name()),
    }
}

impl<T: Table> FromItem for TableRef<T> {}
impl<T: Table> Collectable for TableRef<T> {
    fn collect(&self, w: &mut dyn Write, _: &mut Collector) -> fmt::Result {
        __write_table::<T>(w)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoinType {
    InnerJoin,
    LeftJoin,
    RightJoin,
    FullOuterJoin,
    CrossJoin,
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

    fn join<N: FromItem>(self, with: N, kind: JoinType) -> Join<Self, N> {
        Join {
            l: self,
            r: with,
            conds: Vec::new(),
            kind,
        }
    }

    pub fn left_join<N: FromItem>(self, with: N) -> Join<Self, N> {
        self.join(with, JoinType::LeftJoin)
    }

    pub fn left_join_table<T: Table>(self) -> Join<Self, TableRef<T>> {
        self.left_join(TableRef::new())
    }

    pub fn inner_join<N: FromItem>(self, with: N) -> Join<Self, N> {
        self.join(with, JoinType::InnerJoin)
    }

    pub fn inner_join_table<T: Table>(self) -> Join<Self, TableRef<T>> {
        self.inner_join(TableRef::new())
    }

    pub fn cross_join<N: FromItem>(self, with: N) -> Join<Self, N> {
        self.join(with, JoinType::CrossJoin)
    }

    pub fn cross_join_table<T: Table>(self) -> Join<Self, TableRef<T>> {
        self.cross_join(TableRef::new())
    }

    pub fn full_outer_join<N: FromItem>(self, with: N) -> Join<Self, N> {
        self.join(with, JoinType::FullOuterJoin)
    }

    pub fn full_outer_join_table<T: Table>(self) -> Join<Self, TableRef<T>> {
        self.full_outer_join(TableRef::new())
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
            JoinType::CrossJoin => " CROSS JOIN ",
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
        } else {
            panic!("JOIN ON Conditions must not be empty!");
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

    fn cross_join<F: FromItem>(with: F) -> Join<TableRef<Self>, F> {
        Self::join(JoinType::CrossJoin, with)
    }

    fn cross_join_table<T: Table>() -> Join<TableRef<Self>, TableRef<T>> {
        Self::cross_join(TableRef::new())
    }
}

impl<T> TableJoinExt for T where T: Table {}

impl FromItem for Call {}

pub struct Lateral<T>(pub NamedQuery<T, SelectQuery>);

impl<T: Table> FromItem for Lateral<T> {}
impl<T: Table> Collectable for Lateral<T> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        w.write_str("LATERAL ")?;
        self.0.query._collect(w, t)?;
        w.write_str(" AS ")?;
        w.write_str(T::NAME.name())
    }
}
