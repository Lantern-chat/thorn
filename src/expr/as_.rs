use std::borrow::Cow;

use crate::name::NameError;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenamedExpr<E> {
    inner: E,
    name: &'static str,
}

impl<E> RenamedExpr<E> {
    pub fn reference(&self) -> RenamedExprRef {
        RenamedExprRef { name: self.name }
    }
}

pub trait RenamedExt: ValueExpr + Sized {
    fn rename_as(self, name: &'static str) -> Result<RenamedExpr<Self>, NameError> {
        Ok(RenamedExpr {
            inner: self,
            name: NameError::check_name(name)?,
        })
    }

    fn alias_to<C: Table>(self, column: C) -> RenamedExpr<Self> {
        RenamedExpr {
            inner: self,
            name: column.name(),
        }
    }
}
impl<T> RenamedExt for T where T: ValueExpr {}

impl<E: ValueExpr> ValueExpr for RenamedExpr<E> {}
impl<E: ValueExpr> Expr for RenamedExpr<E> {}
impl<E: ValueExpr> Collectable for RenamedExpr<E> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.inner._collect(w, t)?;
        w.write_str(" AS \"")?;
        w.write_str(self.name)?;
        w.write_str("\"")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenamedExprRef {
    name: &'static str,
}

impl Expr for RenamedExprRef {}
impl Collectable for RenamedExprRef {
    fn collect(&self, w: &mut dyn Write, _: &mut Collector) -> fmt::Result {
        w.write_str("\"")?;
        w.write_str(self.name)?;
        w.write_str("\"")
    }
}
