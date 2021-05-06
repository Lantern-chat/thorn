use super::*;

pub struct CoalesceExpr {
    pub exprs: Vec<Box<dyn Expr>>,
}

impl CoalesceExpr {
    pub fn coalesce<E>(mut self, expr: E) -> Self
    where
        E: Expr + 'static,
    {
        self.exprs.push(Box::new(expr));
        self
    }
}

pub trait CoalesceExt: Expr + Sized {
    fn coalesce<E>(self, expr: E) -> CoalesceExpr
    where
        E: Expr + 'static,
        Self: 'static,
    {
        CoalesceExpr {
            exprs: vec![Box::new(self) as Box<dyn Expr>, Box::new(expr) as Box<dyn Expr>],
        }
    }
}

impl<T> CoalesceExt for T where T: Expr {}

impl Expr for CoalesceExpr {}
impl Collectable for CoalesceExpr {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        w.write_str("COALESCE(")?;

        let mut exprs = self.exprs.iter();

        if let Some(first) = exprs.next() {
            first.collect(w, t)?;
        }

        for e in exprs {
            w.write_str(", ")?;
            e.collect(w, t)?;
        }

        w.write_char(')')
    }
}
