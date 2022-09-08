use crate::{
    query::{update::Value, UpdateQuery},
    table::Column,
    util::collect_delimited,
};

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DoNothing;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DoUpdate;

pub struct DoUpdateSet {
    sets: Vec<(Box<dyn Column>, Value)>,
    wheres: Vec<Box<dyn Expr>>,
}

impl DoUpdate {
    pub fn set<C, V>(self, col: C, value: V) -> DoUpdateSet
    where
        C: Column + 'static,
        V: ValueExpr + 'static,
    {
        DoUpdateSet {
            sets: vec![(Box::new(col), Value::Value(Box::new(value)))],
            wheres: Vec::new(),
        }
    }

    pub fn set_default<C>(self, col: C) -> DoUpdateSet
    where
        C: Column + 'static,
    {
        DoUpdateSet {
            sets: vec![(Box::new(col), Value::Default)],
            wheres: Vec::new(),
        }
    }
}

impl DoUpdateSet {
    pub fn set<C, V>(mut self, col: C, value: V) -> Self
    where
        C: Column + 'static,
        V: ValueExpr + 'static,
    {
        self.sets.push((Box::new(col), Value::Value(Box::new(value))));
        self
    }

    pub fn set_default<C>(mut self, col: C) -> Self
    where
        C: Column + 'static,
    {
        self.sets.push((Box::new(col), Value::Default));
        self
    }

    pub fn and_where<E>(mut self, cond: E) -> Self
    where
        E: BooleanExpr + 'static,
    {
        self.wheres.push(Box::new(cond));
        self
    }
}

impl Collectable for DoNothing {
    fn collect(&self, w: &mut dyn Write, _: &mut Collector) -> fmt::Result {
        w.write_str(" DO NOTHING")
    }
}

impl Collectable for DoUpdateSet {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        w.write_str(" DO UPDATE ")?;

        let mut sets = self.sets.iter();
        if let Some((col, val)) = sets.next() {
            write!(w, "SET \"{}\" = ", col.name())?;
            val.collect(w, t)?;

            for (col, val) in sets {
                write!(w, ", \"{}\" = ", col.name())?;
                val.collect(w, t)?;
            }
        }

        // WHERE
        if !self.wheres.is_empty() {
            w.write_str(" WHERE ")?;
            collect_delimited(&self.wheres, self.wheres.len() > 1, " AND ", w, t)?;
        }

        Ok(())
    }
}

pub enum ConflictAction {
    DoNothing,
    DoUpdateSet(DoUpdateSet),
}

impl From<DoNothing> for ConflictAction {
    fn from(_: DoNothing) -> Self {
        ConflictAction::DoNothing
    }
}

impl From<DoUpdateSet> for ConflictAction {
    fn from(action: DoUpdateSet) -> Self {
        ConflictAction::DoUpdateSet(action)
    }
}

impl Collectable for ConflictAction {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        match self {
            ConflictAction::DoNothing => DoNothing.collect(w, t),
            ConflictAction::DoUpdateSet(dus) => dus.collect(w, t),
        }
    }
}
