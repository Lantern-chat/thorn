use super::*;

#[derive(Default)]
pub struct Case {
    expr: Option<Box<dyn ValueExpr>>,
    branches: Vec<(Box<dyn Expr>, Box<dyn ValueExpr>)>,
    otherwise: Option<Box<dyn ValueExpr>>,
}

impl Case {
    pub fn ternary<E, T, F>(cond: E, on_true: T, on_false: F) -> Self
    where
        E: ValueExpr + 'static,
        T: ValueExpr + 'static,
        F: ValueExpr + 'static,
    {
        Self::default().when(cond, on_true).otherwise(on_false)
    }

    pub fn case<E>(expr: E) -> Self
    where
        E: ValueExpr + 'static,
    {
        let mut case = Self::default();
        case.expr = Some(Box::new(expr));
        case
    }

    pub fn when<C, V>(mut self, equals: C, then: V) -> Self
    where
        C: ValueExpr + 'static,
        V: ValueExpr + 'static,
    {
        assert!(
            self.expr.is_some(),
            "Cannot use by-value case branches without an initial expression to compare to!"
        );

        self.branches.push((Box::new(equals), Box::new(then)));
        self
    }

    pub fn when_condition<C, V>(mut self, cond: C, then: V) -> Self
    where
        C: BooleanExpr + 'static,
        V: ValueExpr + 'static,
    {
        self.branches.push((Box::new(cond), Box::new(then)));
        self
    }

    pub fn otherwise<V>(mut self, value: V) -> Self
    where
        V: ValueExpr + 'static,
    {
        self.otherwise = Some(Box::new(value));
        self
    }
}

impl ValueExpr for Case {}
impl Expr for Case {}

impl Collectable for Case {
    fn needs_wrapping(&self) -> bool {
        true
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        if self.branches.is_empty() && self.otherwise.is_none() {
            panic!("Empty CASE Expression!");
        }

        match self.expr {
            Some(ref expr) => {
                w.write_str("CASE ")?;
                expr._collect(w, t)?
            }
            None => w.write_str("CASE")?,
        }

        for (cond, value) in &self.branches {
            w.write_str(" WHEN ")?;
            cond._collect(w, t)?;
            w.write_str(" THEN ")?;
            value._collect(w, t)?;
        }

        if let Some(ref otherwise) = self.otherwise {
            w.write_str(" ELSE ")?;
            otherwise._collect(w, t)?;
        }

        w.write_str(" END")
    }
}

pub struct If {
    condition: Box<dyn BooleanExpr>,
}

pub struct IfThen {
    case: Case,
}

pub struct IfThenElse {
    case: Case,
}

impl If {
    pub fn condition<B>(cond: B) -> If
    where
        B: BooleanExpr + 'static,
    {
        If {
            condition: Box::new(cond),
        }
    }

    pub fn then<V>(self, value: V) -> IfThen
    where
        V: ValueExpr + 'static,
    {
        IfThen {
            case: Case::default().when_condition(self.condition, value),
        }
    }
}

impl IfThen {
    pub fn otherwise<V>(self, value: V) -> IfThenElse
    where
        V: ValueExpr + 'static,
    {
        IfThenElse {
            case: self.case.otherwise(value),
        }
    }
}

impl ValueExpr for IfThen {}
impl Expr for IfThen {}
impl Collectable for IfThen {
    fn needs_wrapping(&self) -> bool {
        self.case.needs_wrapping()
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.case.collect(w, t)
    }
}

impl ValueExpr for IfThenElse {}
impl Expr for IfThenElse {}
impl Collectable for IfThenElse {
    fn needs_wrapping(&self) -> bool {
        self.case.needs_wrapping()
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.case.collect(w, t)
    }
}
