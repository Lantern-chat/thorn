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

pub trait ValueExpr: Expr {}
impl<T> ValueExpr for T
where
    T: Deref,
    <T as Deref>::Target: ValueExpr,
{
}

pub trait BooleanExpr: ValueExpr {}
impl<T> BooleanExpr for T
where
    T: Deref,
    <T as Deref>::Target: BooleanExpr,
{
}

pub mod as_;
pub mod between;
pub mod binary;
pub mod comparison;
pub mod func;
pub mod is_;
pub mod literal;
pub mod order;
pub mod subquery;
pub mod unary;

pub(crate) mod util;

pub use self::{
    as_::{RenamedExpr, RenamedExt},
    between::{BetweenExpr, BetweenExt},
    binary::{BinaryExpr, BinaryExt},
    comparison::{CompExpr, CompExt},
    func::{Arguments, Builtin, Call},
    is_::{IsExpr, IsExt},
    literal::Literal,
    order::{OrderExpr, OrderExt},
    subquery::{ExistsExpr, ExistsExt, Subquery, SubqueryExt},
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

impl ValueExpr for PlaceholderExpr {}
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

pub struct Any;
impl ValueExpr for Any {}
impl Expr for Any {}
impl Collectable for Any {
    fn collect(&self, w: &mut dyn Write, _: &mut Collector) -> fmt::Result {
        w.write_str("*")
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnRef<C>(pub C);

impl<C> ValueExpr for ColumnRef<C> where C: Table {}
impl<C> Expr for ColumnRef<C> where C: Table {}
impl<C> Collectable for ColumnRef<C>
where
    C: Table,
{
    fn collect(&self, w: &mut dyn Write, _: &mut Collector) -> fmt::Result {
        write!(w, r#""{}"."{}""#, C::NAME.name(), self.0.name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Subscript<E, I> {
    inner: E,
    index: I,
}

impl<E: ValueExpr, I: ValueExpr> ValueExpr for Subscript<E, I> {}
impl<E: ValueExpr, I: ValueExpr> Expr for Subscript<E, I> {}
impl<E: ValueExpr, I: ValueExpr> Collectable for Subscript<E, I> {
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
impl<E: ValueExpr> ValueExpr for Field<E, &'static str> {}
impl<E: ValueExpr> Expr for Field<E, &'static str> {}
impl<E: ValueExpr> Collectable for Field<E, &'static str> {
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

impl<T> CastExt for T where T: ValueExpr {}

impl<T: ValueExpr> ValueExpr for CastExpr<T> {}
impl<T: ValueExpr> Expr for CastExpr<T> {}
impl<T: ValueExpr> Collectable for CastExpr<T> {
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

impl<E: ValueExpr> BooleanExpr for LikeExpr<E> {}
impl<E: ValueExpr> ValueExpr for LikeExpr<E> {}
impl<E: ValueExpr> Expr for LikeExpr<E> {}
impl<E: ValueExpr> Collectable for LikeExpr<E> {
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

/*
macro_rules! impl_tuple_expr {
    ($(($($t:ident),*)),*$(,)*) => {
        $(
            impl<$($t: Expr),*> Expr for ($($t,)*) {}
            impl<$($t: Collectable),*> Collectable for ($($t,)*) {
                fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
                    let ($(ref $t,)*) = *self;
                    $(
                        $t._collect(w, t)?;
                    )*
                    Ok(())
                }
            }
        )*
    }
}

impl_tuple_expr! {
    (A),
    //(A, B),
    //(A, B, C),
    //(A, B, C, D),
    //(A, B, C, D, E),
    //(A, B, C, D, E, F),
    //(A, B, C, D, E, F, G),
    //(A, B, C, D, E, F, G, H),
    //(A, B, C, D, E, F, G, H, I),
    //(A, B, C, D, E, F, G, H, I, J),
    //(A, B, C, D, E, F, G, H, I, J, K),
    //(A, B, C, D, E, F, G, H, I, J, K, L),
    //(A, B, C, D, E, F, G, H, I, J, K, L, M),
    //(A, B, C, D, E, F, G, H, I, J, K, L, M, N),
}
*/
