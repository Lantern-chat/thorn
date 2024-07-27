use crate::{
    collect::{Collectable, Collector},
    conflict::ConflictAction,
    name::NameError,
    order::Order,
    table::Column,
    *,
};

use std::fmt::{self, Write};

use super::{
    from_item::*,
    with::{NamedQuery, WithQuery, WithableQuery},
    FromItem,
};

enum Values {
    Values(Vec<Box<dyn ValueExpr>>),
    Query(Box<dyn ValueExpr>),
}

enum Conflict {
    None,
    Constraint(&'static str),
    // TODO: Support collations and where statements
    Column(Vec<Box<dyn Column>>),
    Expr(Vec<Box<dyn Expr>>),
}

pub struct InsertQuery<T> {
    pub(crate) with: Option<WithQuery>,
    cols: Vec<T>,
    values: Values,
    returning: Vec<Box<dyn Expr>>,
    conflict_target: Conflict,
    conflict_action: Option<ConflictAction>,
}

impl<T> Default for InsertQuery<T> {
    fn default() -> Self {
        InsertQuery {
            with: Default::default(),
            cols: Default::default(),
            values: Values::Values(Vec::new()),
            returning: Default::default(),
            conflict_target: Conflict::None,
            conflict_action: None,
        }
    }
}

impl InsertQuery<()> {
    pub fn with<W: Table, Q>(mut self, query: NamedQuery<W, Q>) -> Self
    where
        Q: WithableQuery + 'static,
    {
        self.with = Some(match self.with {
            Some(with) => with.with(query),
            None => Query::with().with(query),
        });

        self
    }

    pub fn into<T: Table>(self) -> InsertQuery<T> {
        InsertQuery {
            with: self.with,
            ..InsertQuery::<T>::default()
        }
    }
}

impl<T: Table> InsertQuery<T> {
    pub fn cols<'a>(mut self, cols: impl IntoIterator<Item = &'a T>) -> Self {
        self.cols.extend(cols.into_iter().cloned());
        self
    }

    pub fn values<E>(mut self, exprs: impl IntoIterator<Item = E>) -> Self
    where
        E: ValueExpr + 'static,
    {
        match self.values {
            Values::Values(ref mut values) => {
                values.extend(exprs.into_iter().map(|e| Box::new(e) as Box<dyn ValueExpr>));
            }
            Values::Query(_) => panic!("Cannot insert both values and query results!"),
        }

        self
    }

    pub fn value<E>(mut self, expr: E) -> Self
    where
        E: ValueExpr + 'static,
    {
        match self.values {
            Values::Values(ref mut values) => values.push(Box::new(expr)),
            Values::Query(_) => panic!("Cannot insert both values and query results!"),
        }

        self
    }

    pub fn query<E>(mut self, expr: E) -> Self
    where
        E: ValueExpr + 'static,
    {
        self.values = match self.values {
            Values::Values(ref values) if values.is_empty() => Values::Query(Box::new(expr)),
            Values::Query(_) => panic!("Cannot insert more than one query!"),
            Values::Values(_) => panic!("Cannot insert both values and query results!"),
        };
        self
    }

    pub fn on_conflict<C>(mut self, cols: impl IntoIterator<Item = C>, action: impl Into<ConflictAction>) -> Self
    where
        C: Column + 'static,
    {
        self.conflict_target =
            Conflict::Column(cols.into_iter().map(|c| Box::new(c) as Box<dyn Column>).collect());
        self.conflict_action = Some(action.into());
        self
    }

    pub fn on_constraint_conflict(
        mut self,
        constraint_name: &'static str,
        action: impl Into<ConflictAction>,
    ) -> Result<Self, NameError> {
        let name = NameError::check_name(constraint_name)?;

        self.conflict_target = Conflict::Constraint(name);
        self.conflict_action = Some(action.into());

        Ok(self)
    }

    pub fn on_expr_conflict<E>(
        mut self,
        targets: impl IntoIterator<Item = E>,
        action: impl Into<ConflictAction>,
    ) -> Self
    where
        E: Expr + 'static,
    {
        self.conflict_target = Conflict::Expr(targets.into_iter().map(|e| Box::new(e) as Box<dyn Expr>).collect());
        self.conflict_action = Some(action.into());
        self
    }

    pub fn returning<E>(mut self, expr: E) -> Self
    where
        E: Expr + 'static,
    {
        self.returning.push(Box::new(expr));
        self
    }
}

impl<T: Table> Collectable for InsertQuery<T> {
    fn needs_wrapping(&self) -> bool {
        true
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        use crate::expr::util::collect_delimited;

        if let Some(ref with) = self.with {
            with._collect(w, t)?;
            w.write_str(" ")?; // space before INSERT
        }

        w.write_str("INSERT INTO ")?;

        TableRef::<T>::new()._collect(w, t)?;

        // print column names without table prefix
        let mut cols = self.cols.iter();
        if let Some(col) = cols.next() {
            w.write_str(" (\"")?;
            w.write_str(col.name())?;

            for col in cols {
                w.write_str("\", \"")?;
                w.write_str(col.name())?;
            }
            w.write_str("\")")?;
        }

        match self.values {
            Values::Values(ref values) => {
                if values.is_empty() {
                    w.write_str(" DEFAULT VALUES")?;
                } else {
                    if !self.cols.is_empty() {
                        assert_eq!(
                            values.len(),
                            self.cols.len(),
                            "Columns and Values must be equal length!"
                        );
                    }

                    w.write_str(" VALUES ")?;
                    collect_delimited(values, true, ", ", w, t)?;
                }
            }
            Values::Query(ref query) => query._collect(w, t)?,
        }

        if let Some(ref conflict_action) = self.conflict_action {
            w.write_str(" ON CONFLICT ")?;

            match self.conflict_target {
                Conflict::None => {}
                Conflict::Constraint(constraint) => write!(w, "ON CONSTRAINT \"{}\"", constraint)?,
                Conflict::Column(ref cols) => {
                    w.write_str("(")?;

                    let mut cols = cols.iter();

                    match cols.next() {
                        Some(first) => write!(w, "\"{}\"", first.name())?,
                        None => panic!("Missing conflict targets for insert!"),
                    }

                    for col in cols {
                        write!(w, ", \"{}\"", col.name())?;
                    }

                    w.write_str(")")?;
                }
                Conflict::Expr(ref exprs) => {
                    w.write_str("(")?;

                    let mut exprs = exprs.iter();

                    match exprs.next() {
                        Some(first) => first._collect(w, t)?,
                        None => panic!("Missing conflict targets for insert!"),
                    }

                    for expr in exprs {
                        w.write_str(", ")?;
                        expr._collect(w, t)?;
                    }

                    w.write_str(")")?;
                }
            }

            conflict_action.collect(w, t)?;
        }

        if !self.returning.is_empty() {
            w.write_str(" RETURNING ")?;
            collect_delimited(&self.returning, false, ", ", w, t)?;
        }

        Ok(())
    }
}
