use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AccessOp {
    Array,
    Json,
    JsonText,
    JsonSubObject,
    JsonSubObjectText,
}

pub struct Access<E, I> {
    value: E,
    idx: I,
    op: AccessOp,
}

impl<E: ValueExpr, I: ValueExpr> ValueExpr for Access<E, I> {}
impl<E: ValueExpr, I: ValueExpr> Expr for Access<E, I> {}
impl<E: ValueExpr, I: ValueExpr> Collectable for Access<E, I> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.value._collect(w, t)?;

        w.write_str(match self.op {
            AccessOp::Array => "[",
            AccessOp::Json => "->",
            AccessOp::JsonText => "->>",
            AccessOp::JsonSubObject => "#>",
            AccessOp::JsonSubObjectText => "#>>",
        })?;

        self.idx._collect(w, t)?;

        if self.op == AccessOp::Array {
            w.write_str("]")?;
        }

        Ok(())
    }
}

impl<E, I> Access<E, I> {
    const fn new(value: E, idx: I, op: AccessOp) -> Self {
        Access { value, idx, op }
    }
}

pub trait AccessExt: ValueExpr + Sized {
    fn array_index<I: ValueExpr + 'static>(self, idx: I) -> Access<Self, I> {
        Access::new(self, idx, AccessOp::Array)
    }

    fn json_extract<I: ValueExpr + 'static>(self, idx: I) -> Access<Self, I> {
        Access::new(self, idx, AccessOp::Json)
    }

    fn json_extract_as_text<I: ValueExpr + 'static>(self, idx: I) -> Access<Self, I> {
        Access::new(self, idx, AccessOp::JsonText)
    }

    fn json_extract_subobject<I: ValueExpr + 'static>(self, idx: I) -> Access<Self, I> {
        Access::new(self, idx, AccessOp::JsonSubObject)
    }

    fn json_extract_subobject_text<I: ValueExpr + 'static>(self, idx: I) -> Access<Self, I> {
        Access::new(self, idx, AccessOp::JsonSubObjectText)
    }
}

impl<T> AccessExt for T where T: ValueExpr {}
