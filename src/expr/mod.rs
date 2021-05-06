//use std::io::{self, Write};

use std::{
    collections::{btree_map::Entry, BTreeMap},
    fmt::{self, Write},
};

use pg::Type;

use crate::{
    collect::{Collectable, Collector},
    ty::TypeExt,
    Table,
};

const _: Option<&dyn Expr> = None;
pub trait Expr: Collectable {}

pub mod between;
pub mod binary;
pub mod coalesce;
pub mod is_;
pub mod literal;

pub use self::{
    between::{BetweenExpr, BetweenExt},
    binary::{BinaryExpr, BinaryExt},
    coalesce::{CoalesceExpr, CoalesceExt},
    is_::{IsExpr, IsExt},
    literal::Literal,
};

pub type Var = PlaceholderExpr;

#[derive(Debug, Clone, PartialEq)]
pub struct PlaceholderExpr {
    pub ty: Type,
    pub idx: Option<usize>,
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
    pub inner: E,
    pub index: I,
}

impl<E: Expr, I: Expr> Expr for Subscript<E, I> {}
impl<E: Expr, I: Expr> Collectable for Subscript<E, I> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.inner._collect(w, t)?;
        w.write_char('[')?;
        self.index.collect(w, t)?; // already enclosed by delimiters, so wrapping is unnecessary
        w.write_char(']')
    }
}

pub struct Field<E, N> {
    pub inner: E,
    pub field: N,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Not,
    BitNot,
    Abs,
    SquareRoot,
    CubeRoot,
}

pub struct UnaryExpr<V> {
    pub value: V,
    pub op: UnaryOp,
}

impl<V> Expr for UnaryExpr<V> where V: Expr {}
impl<V> Collectable for UnaryExpr<V>
where
    V: Expr,
{
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        w.write_str(match self.op {
            UnaryOp::Not => "!",
            UnaryOp::BitNot => "~",
            UnaryOp::Abs => "@",
            UnaryOp::SquareRoot => "|/",
            UnaryOp::CubeRoot => "||/",
        })?;

        self.value._collect(w, t)
    }

    fn needs_wrapping(&self) -> bool {
        true
    }
}

pub struct CastExpr<T> {
    pub inner: T,
    pub into: Type,
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
