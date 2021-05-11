use super::*;

macro_rules! decl_builtins {
    ($($name:ident),*$(,)*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Builtin {
            $($name),*
        }

        impl Builtin {
            pub fn name(self) -> &'static str {
                paste::paste! {
                    match self {
                        $(Builtin::$name => stringify!([<$name:snake:upper>])),*
                    }
                }
            }
        }
    }
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

    pub fn arg<E>(self, arg: E) -> Call
    where
        E: Expr + 'static,
    {
        self.call().arg(arg)
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
            CallName::Builtin(b) => b.name(),
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

decl_builtins! {
    Abs,
    Sqrt,
    Cbrt,
    Degrees,
    Radians,
    Div,
    Exp,
    Ceil,
    Floor,
    Round,
    Ln,
    Log,
    Log10,
    Mod,
    Pi,
    Power,
    Sign,
    Trunc,
    Factorial,
    Gcd,
    Lcm,
    WidthBucket,
    Random,
    Setseed,
    MinScale,

    Sin,
    Cos,
    Tan,
    Cot,
    Acos,
    Asin,
    Atan,
    Atan2,

    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,

    Concat,
    CharLength,
    Length,
    Lower,
    Upper,
    Left,
    Right,
    Lpad,
    Ltrim,
    Rpad,
    Rtrim,
    StartsWith,
    SplitPart,
    ToAscii,
    Chr,
    Ascii,
    Btrim,
    Encode,
    Decode,
    Md5,
    Translate,

    OctetLength,
    BitLength,
    GetBit,
    GetByte,
    SetBit,
    SetByte,
    Sha224,
    Sha256,
    Sha384,
    Sha512,
    Substr,

    ToChar,
    ToDate,
    ToNumber,
    ToTimestamp,

    Now,
    ClockTimestamp,
    CurrentDate,
    CurrentTime,
    CurrentTimestamp,

    Sum,
    Min,
    Max,
    Avg,
    Stddev,
    StddevPop,
    StddevSamp,
    Variance,
    VarPop,
    VarSamp,

    BitAnd,
    BitOr,
    BoolAnd,
    BoolOr,
    Every,
    Count,
}
