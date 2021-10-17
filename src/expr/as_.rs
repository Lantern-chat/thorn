use std::borrow::Cow;

use super::*;

pub struct RenamedExpr<E> {
    inner: E,
    name: &'static str,
}

impl<E> RenamedExpr<E> {
    pub fn reference(&self) -> RenamedExprRef {
        RenamedExprRef { name: self.name }
    }
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum RenameError {
    #[error("Names must be at least 1 character long!")]
    NameTooShort,

    #[error("Names must start with an alphabetic character!")]
    NonAlphaStart,

    #[error("Names must only contain alphanumeric characters!")]
    InvalidName,
}

fn valid_name_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn valid_name_char(c: char) -> bool {
    c.is_alphanumeric() || ['_', '$'].contains(&c)
}

impl RenameError {
    pub(crate) fn check_name(name: &'static str) -> Result<&'static str, Self> {
        let mut chars = name.chars();

        match chars.next() {
            None => return Err(RenameError::NameTooShort),
            Some(c) if !valid_name_start(c) => return Err(RenameError::NonAlphaStart),
            _ => {}
        }

        if !chars.all(valid_name_char) {
            return Err(RenameError::InvalidName);
        }

        Ok(name)
    }
}

pub trait RenamedExt: ValueExpr + Sized {
    fn rename_as(self, name: &'static str) -> Result<RenamedExpr<Self>, RenameError> {
        Ok(RenamedExpr {
            inner: self,
            name: RenameError::check_name(name)?,
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
