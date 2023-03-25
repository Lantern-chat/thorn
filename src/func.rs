pub trait Func {
    const NAME: &'static str;
    const NUM_PARAMS: usize;
}

#[macro_export]
macro_rules! functions {
    ($(
        $(#[$meta:meta])*
        $vis:vis extern "pg" fn $name:ident ($($arg:ident $(:$ty:expr)?),*) $(in $schema:ident)?;
    )*) => {$crate::paste::paste!{$(
        #[allow(non_camel_case_types)]
        pub struct $name;

        impl $crate::func::Func for $name {
            const NAME: &'static str = concat!($(stringify!([<$schema:snake>]), ".",)? stringify!($name));
            const NUM_PARAMS: usize = $crate::functions!(@COUNT $($arg)*);
        }

        #[allow(clippy::too_many_arguments)]
        impl $name {
            $(#[$meta])*
            $vis fn call($($arg: impl $crate::ValueExpr + 'static),*) -> $crate::Call {
                use $crate::{func::Func, expr::CastExt};

                $crate::Call::custom(<Self as Func>::NAME).args(($($arg$(.cast($ty))?,)*))
            }
        }
    )*}};

    (@COUNT $arg:ident $($rest:ident)*) => {
        1 + $crate::functions!(@COUNT $($rest)*)
    };

    (@COUNT) => { 0 };
}

functions! {
    pub extern "pg" fn test_fn(v: pg::Type::TEXT) in TestSchema;
}
