use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Builtin {
    Min,
    Max,
    Sum,
}

enum CallName {
    Builtin(Builtin),
    Custom(&'static str),
}

pub struct Call {
    name: CallName,
    args: Vec<Box<dyn Expr>>,
}

impl Builtin {
    pub fn call(self) -> Call {
        Call {
            name: CallName::Builtin(self),
            args: Vec::new(),
        }
    }

    fn to_str(self) -> &'static str {
        match self {
            Builtin::Min => "MIN",
            Builtin::Max => "MAX",
            Builtin::Sum => "SUM",
        }
    }
}

impl Call {
    pub fn custom(name: &'static str) -> Self {
        Call {
            name: CallName::Custom(name),
            args: Vec::new(),
        }
    }

    pub fn arg<E>(mut self, arg: E) -> Self
    where
        E: Expr + 'static,
    {
        self.args.push(Box::new(arg));
        self
    }

    pub fn args<E>(mut self, args: impl IntoIterator<Item = E>) -> Self
    where
        E: Expr + 'static,
    {
        self.args
            .extend(args.into_iter().map(|e| Box::new(e) as Box<dyn Expr>));
        self
    }
}

impl Expr for Call {}
impl Collectable for Call {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        let name = match self.name {
            CallName::Builtin(b) => b.to_str(),
            CallName::Custom(name) => name,
        };
        write!(w, "{}(", name)?;

        let mut args = self.args.iter();
        if let Some(arg) = args.next() {
            arg._collect(w, t)?;
        }
        for arg in args {
            w.write_str(", ")?;
            arg._collect(w, t)?;
        }

        w.write_str(")")
    }
}
