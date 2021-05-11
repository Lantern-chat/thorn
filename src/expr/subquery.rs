use crate::query::SelectQuery;

use super::*;

pub struct ExistsExpr {
    query: SelectQuery,
}

pub trait ExistsExt {
    fn exists(self) -> ExistsExpr;
}

impl ExistsExt for SelectQuery {
    fn exists(self) -> ExistsExpr {
        ExistsExpr { query: self }
    }
}

impl Expr for ExistsExpr {}
impl Collectable for ExistsExpr {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        w.write_str("EXISTS ")?;
        self.query._collect(w, t) // paren wraps automatically
    }
}

enum Kind {
    Any,
    All,
}

pub struct Subquery {
    query: SelectQuery,
    kind: Kind,
}

impl comparison::ComparableExpr for Subquery {}

pub trait SubqueryExt {
    fn all(self) -> Subquery;
    fn any(self) -> Subquery;
}

impl SubqueryExt for SelectQuery {
    fn all(self) -> Subquery {
        Subquery {
            query: self,
            kind: Kind::All,
        }
    }

    fn any(self) -> Subquery {
        Subquery {
            query: self,
            kind: Kind::Any,
        }
    }
}

impl Collectable for Subquery {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        w.write_str(match self.kind {
            Kind::Any => "ANY ",
            Kind::All => "ALL ",
        })?;

        self.query._collect(w, t) // paren wraps automatically
    }
}

impl Subquery {
    pub fn less_than<E: Expr>(self, expr: E) -> CompExpr<E, Self> {
        expr.greater_than_equal(self)
    }

    pub fn less_than_equal<E: Expr>(self, expr: E) -> CompExpr<E, Self> {
        expr.greater_than(self)
    }

    pub fn greater_than<E: Expr>(self, expr: E) -> CompExpr<E, Self> {
        expr.less_than_equal(self)
    }

    pub fn greater_than_equal<E: Expr>(self, expr: E) -> CompExpr<E, Self> {
        expr.less_than(self)
    }

    pub fn equals<E: Expr>(self, expr: E) -> CompExpr<E, Self> {
        expr.equals(self)
    }

    pub fn not_equals<E: Expr>(self, expr: E) -> CompExpr<E, Self> {
        expr.not_equals(self)
    }
}
