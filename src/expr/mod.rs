//use std::io::{self, Write};

use std::{
    collections::{btree_map::Entry, BTreeMap},
    fmt::{self, Write},
    ops::Deref,
};

use pg::Type;

use crate::{
    collect::{Collectable, Collector},
    ty::TypeExt,
    Table,
};

const _: Option<&dyn Expr> = None;
pub trait Expr: Collectable {}

impl<T> Expr for T
where
    T: Deref,
    <T as Deref>::Target: Expr,
{
}

pub mod as_;
pub mod between;
pub mod binary;
pub mod coalesce;
pub mod is_;
pub mod literal;
pub mod order;
pub mod unary;

pub use self::{
    between::{BetweenExpr, BetweenExt},
    binary::{BinaryExpr, BinaryExt},
    coalesce::{CoalesceExpr, CoalesceExt},
    is_::{IsExpr, IsExt},
    literal::Literal,
    order::{OrderExpr, OrderExt},
    unary::{UnaryExpr, UnaryExt},
};

pub type Var = PlaceholderExpr;

#[derive(Debug, Clone, PartialEq)]
pub struct PlaceholderExpr {
    ty: Type,
    idx: Option<usize>,
}

impl PlaceholderExpr {
    #[inline]
    pub const fn of(ty: Type) -> Self {
        PlaceholderExpr { ty, idx: None }
    }

    #[inline]
    pub const fn at(ty: Type, idx: usize) -> Self {
        PlaceholderExpr { ty, idx: Some(idx) }
    }
}

impl Expr for PlaceholderExpr {}
impl Collectable for PlaceholderExpr {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        let idx = match self.idx {
            Some(idx) => {
                t.insert(idx, self.ty.clone());
                idx
            }
            None => t.push(self.ty.clone()),
        };

        write!(w, "${}", idx)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnRef<C>(pub C);

impl<C> Expr for ColumnRef<C> where C: Table {}
impl<C> Collectable for ColumnRef<C>
where
    C: Table,
{
    fn collect(&self, w: &mut dyn Write, _: &mut Collector) -> fmt::Result {
        write!(w, r#""{}"."{}""#, C::NAME, self.0.name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Subscript<E, I> {
    inner: E,
    index: I,
}

impl<E: Expr, I: Expr> Expr for Subscript<E, I> {}
impl<E: Expr, I: Expr> Collectable for Subscript<E, I> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.inner._collect(w, t)?;
        w.write_str("[")?;
        self.index.collect(w, t)?; // already enclosed by delimiters, so wrapping is unnecessary
        w.write_str("]")
    }
}

pub struct Field<E, N> {
    inner: E,
    field: N,
}

// TODO: Other strongly-typed named fields?
impl<E> Expr for Field<E, &'static str> where E: Expr {}
impl<E> Collectable for Field<E, &'static str>
where
    E: Expr,
{
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.inner._collect(w, t)?;
        write!(w, ".\"{}\"", self.field)
    }
}

pub struct CastExpr<T> {
    inner: T,
    into: Type,
}

pub trait CastExt: Expr + Sized {
    fn cast(self, into: Type) -> CastExpr<Self> {
        CastExpr { inner: self, into }
    }
}

impl<T> CastExt for T where T: Expr {}

impl<T> Expr for CastExpr<T> where T: Expr {}
impl<T> Collectable for CastExpr<T>
where
    T: Expr,
{
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.inner._collect(w, t)?;
        write!(w, "::{}", self.into.name())
    }
}

pub struct LikeExpr<E> {
    inner: E,
    not: bool,
    similar: bool,
    pattern: Literal,
}

#[rustfmt::skip]
pub trait LikeExt: Expr + Sized {
    fn like(self, pattern: &'static str) -> LikeExpr<Self> {
        LikeExpr { inner: self, not: false, similar: false, pattern: Literal::TextStr(pattern) }
    }
    fn not_like(self, pattern: &'static str) -> LikeExpr<Self> {
        LikeExpr { inner: self, not: true, similar: false, pattern: Literal::TextStr(pattern) }
    }
    fn similar_to(self, pattern: &'static str) -> LikeExpr<Self> {
        LikeExpr { inner: self, not: false, similar: true, pattern: Literal::TextStr(pattern) }
    }
    fn not_similar_to(self, pattern: &'static str) -> LikeExpr<Self> {
        LikeExpr { inner: self, not: true, similar: true, pattern: Literal::TextStr(pattern) }
    }
}
impl<T> LikeExt for T where T: Expr {}

impl<E: Expr> Expr for LikeExpr<E> {}
impl<E: Expr> Collectable for LikeExpr<E> {
    fn needs_wrapping(&self) -> bool {
        true
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.inner._collect(w, t)?;

        w.write_str(match (self.similar, self.not) {
            (false, false) => " LIKE ",
            (false, true) => " NOT LIKE ",
            (true, false) => " SIMILAR TO ",
            (true, true) => " NOT SIMILAR TO ",
        })?;

        self.pattern._collect(w, t)
    }
}
