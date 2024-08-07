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

pub mod access;
pub mod as_;
pub mod between;
pub mod binary;
pub mod bits;
pub mod case;
pub mod comparison;
pub mod conflict;
pub mod func;
pub mod in_;
pub mod is_;
pub mod literal;
pub mod order;
pub mod subquery;
pub mod unary;
pub mod window;

pub(crate) mod util;

pub use self::{
    access::{Access, AccessExt},
    as_::{RenamedExpr, RenamedExt},
    between::{BetweenExpr, BetweenExt},
    binary::{BinaryExpr, BinaryExt},
    bits::BitsExt,
    case::{Case, If},
    comparison::{CompExpr, CompExt},
    conflict::{DoNothing, DoUpdate},
    func::{Arguments, Builtin, Call},
    in_::InExt,
    is_::{IsExpr, IsExt},
    literal::{Lit, Literal},
    order::{OrderExpr, OrderExt},
    subquery::{ExistsExpr, ExistsExt, Subquery, SubqueryExt},
    unary::{UnaryExpr, UnaryExt},
    window::{WindowExpr, WindowExt},
};

pub type Var = PlaceholderExpr;

#[derive(Debug, Clone, PartialEq)]
pub struct PlaceholderExpr {
    ty: Type,
    idx: Option<usize>,
}

#[rustfmt::skip]
impl PlaceholderExpr {
    #[inline]
    pub fn of(ty: impl Into<Type>) -> Self {
        PlaceholderExpr { ty: ty.into(), idx: None }
    }

    #[inline]
    pub fn at(ty: impl Into<Type>, idx: usize) -> Self {
        PlaceholderExpr { ty: ty.into(), idx: Some(idx) }
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
    pattern: Lit<&'static str>,
}

#[rustfmt::skip]
pub trait LikeExt: Expr + Sized {
    fn like(self, pattern: &'static str) -> LikeExpr<Self> {
        LikeExpr { inner: self, not: false, similar: false, pattern: Lit(pattern) }
    }
    fn not_like(self, pattern: &'static str) -> LikeExpr<Self> {
        LikeExpr { inner: self, not: true, similar: false, pattern: Lit(pattern) }
    }
    fn similar_to(self, pattern: &'static str) -> LikeExpr<Self> {
        LikeExpr { inner: self, not: false, similar: true, pattern: Lit(pattern) }
    }
    fn not_similar_to(self, pattern: &'static str) -> LikeExpr<Self> {
        LikeExpr { inner: self, not: true, similar: true, pattern: Lit(pattern) }
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

macro_rules! forward_binary_ops {
    (@OP $name:ident<$($generic:ident: $bound:ident),*> $op_trait:ident::$op:ident => $method:ident) => {
        impl std::ops::$op_trait for
    };

    ($name:ident $(<$($generic:ident:$bound:ident),*>)?) => {
        forward_binary_ops!(@OP $name<$($($generic: $bound),*)?> Add::add);
        forward_binary_ops!(@OP $name<$($($generic: $bound),*)?> Sub::sub);
        forward_binary_ops!(@OP $name<$($($generic: $bound),*)?> Add::add);
        forward_binary_ops!(@OP $name<$($($generic: $bound),*)?> Add::add);
        forward_binary_ops!(@OP $name<$($($generic: $bound),*)?> Add::add);
        forward_binary_ops!(@OP $name<$($($generic: $bound),*)?> Add::add);
    };
}
