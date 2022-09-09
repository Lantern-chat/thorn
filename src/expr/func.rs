use super::*;

use std::borrow::Cow;

macro_rules! decl_builtins {
    ($($name:ident),*$(,)*) => {paste::paste! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Builtin {
            $($name),*
        }

        impl Builtin {
            pub fn name(self) -> &'static str {
                match self {
                    $(Builtin::$name => stringify!([<$name:snake:upper>])),*
                }
            }

            $(
                #[doc = "Constructs an expression for the `" [<$name:snake:upper>] "` built-in function"]
                pub fn [<$name:snake>]<T>(args: T) -> Call where T: Arguments {
                    Builtin::$name.args(args)
                }
            )*
        }
    }}
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CallName {
    Builtin(Builtin),
    Custom(Cow<'static, str>),
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

    pub fn args<T>(self, args: T) -> Call
    where
        T: Arguments,
    {
        self.call().args(args)
    }
}

pub trait Arguments {
    fn to_vec(self) -> Vec<Box<dyn Expr>>;
}

impl Arguments for () {
    fn to_vec(self) -> Vec<Box<dyn Expr>> {
        Vec::new()
    }
}

impl Call {
    pub fn custom(name: impl Into<Cow<'static, str>>) -> Self {
        Call {
            name: CallName::Custom(name.into()),
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

    pub fn extend_args(mut self, args: impl IntoIterator<Item = Box<dyn Expr>>) -> Self {
        self.args.extend(args);
        self
    }

    pub fn args<T>(mut self, args: T) -> Self
    where
        T: Arguments,
    {
        self.args.extend(args.to_vec());
        self
    }
}

impl ValueExpr for Call {}
impl Expr for Call {}
impl Collectable for Call {
    #[rustfmt::skip]
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        if let CallName::Builtin(Builtin::Array) = self.name {
            w.write_str("ARRAY[")?;
        } else {
            write!(w, "{}(", match self.name {
                CallName::Builtin(b) => b.name(),
                CallName::Custom(ref name) => &*name,
            })?;
        }

        let mut args = self.args.iter();
        if let Some(arg) = args.next() {
            arg._collect(w, t)?;
        }
        for arg in args {
            w.write_str(", ")?;
            arg._collect(w, t)?;
        }

        w.write_str(match self.name {
            CallName::Builtin(Builtin::Array) => "]",
            _ => ")",
        })
    }
}

decl_builtins! {
    Array,
    Coalesce,
    Nullif,
    Any,

    Greatest,
    Least,

    Degrees,
    Radians,
    Exp,
    Ceil,
    Floor,
    Round,
    Ln,
    Log,
    Log10,
    Pi,
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

    ArrayAgg,
    StringAgg,

    RowNumber,
    Rank,
    DenseRank,
    PercentRank,
    CumeDist,
    Ntile,
    Lag,
    Lead,
    FirstValue,
    LastValue,
    NthValue,

    GenerateSeries,
    GenerateSubscripts,

    Unnest,
    ArrayLength,
    ArrayCat,
    ArrayAppend,
    ArrayDims,
    ArrayFill,
    ArrayLower,
    ArrayNdims,
    ArrayPosition,
    ArrayPositions,
    ArrayPrepend,
    ArrayRemove,
    ArrayReplace,
    ArrayUpper,
    ArrayToString,
    Cardinality,
    TrimArray,
}

impl Builtin {
    /// Shorthand for `COALESCE(ARRAY_AGG(args), '{}')`
    pub fn array_agg_nonnull<T>(args: T) -> Call
    where
        T: Arguments,
    {
        Builtin::coalesce((Builtin::array_agg(args), Literal::EMPTY_ARRAY.clone()))
    }
}

// TODO: Figure out a better way to do this
macro_rules! impl_args {
    ($(($($t:ident),*)),*$(,)*) => {
        $(
            #[allow(non_snake_case)]
            impl<$($t: Expr + 'static),*> Arguments for ($($t,)*) {
                fn to_vec(self) -> Vec<Box<dyn Expr>> {
                    let ($($t,)*) = self;
                    vec![$(Box::new($t)),*]
                }
            }
        )*
    };
}

macro_rules! impl_arg_for_exprs {
    ($($t:ident$(<$($g:ident),*>)?),*$(,)*) => {
        $(
            impl$(<$($g),*>)? Arguments for $t$(<$($g),*>)? where Self: Expr + 'static {
                fn to_vec(self) -> Vec<Box<dyn Expr>> {
                    vec![Box::new(self)]
                }
            }
        )*
    };
}

macro_rules! impl_arg_for_literals {
    ($($t:ty),*) => {
        $(
            impl Arguments for $t {
                fn to_vec(self) -> Vec<Box<dyn Expr>> {
                    vec![Box::new(self.lit())]
                }
            }
        )*
    };
}

impl_args! {
    (A),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F),
    (A, B, C, D, E, F, G),
    (A, B, C, D, E, F, G, H),
    (A, B, C, D, E, F, G, H, I),
    (A, B, C, D, E, F, G, H, I, J),
    (A, B, C, D, E, F, G, H, I, J, K),
    (A, B, C, D, E, F, G, H, I, J, K, L),
    (A, B, C, D, E, F, G, H, I, J, K, L, M),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N),
}

impl_arg_for_exprs! {
    Any,
    Var,
    BetweenExpr<X, A, B>,
    BinaryExpr<Lhs, Rhs>,
    CompExpr<Lhs, Rhs>,
    Call,
    IsExpr<V>,
    Literal,
    OrderExpr<E>,
    ExistsExpr,
    UnaryExpr<V>,
    Subscript<E, I>,
    CastExpr<T>,
    LikeExpr<E>,
    Field<E, I>,
    Case,
}

impl_arg_for_literals!(i8, i16, i32, i64, bool, &'static str, f32, f64, String);

impl<T> Arguments for Vec<T>
where
    T: Expr + 'static,
{
    fn to_vec(self) -> Vec<Box<dyn Expr>> {
        self.into_iter().map(|x| Box::new(x) as Box<dyn Expr>).collect()
    }
}

impl<C> Arguments for ColumnRef<C>
where
    C: Table,
{
    fn to_vec(self) -> Vec<Box<dyn Expr>> {
        vec![Box::new(self)]
    }
}
