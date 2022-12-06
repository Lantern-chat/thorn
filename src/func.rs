#[macro_export]
macro_rules! functions {
    ($(
        $(#[$meta:meta])*
        $vis:vis extern "pg" fn $name:ident ($($arg:ident $(:$ty:expr)?),*) $(in $schema:ident)?;
    )*) => {paste::paste!{$(
        $(#[$meta])*
        $vis fn [<$name:snake>]($($arg: impl $crate::ValueExpr + 'static),*) -> $crate::Call {
            use $crate::expr::CastExt;

            $crate::Call::custom(concat!($(stringify!([<$schema:snake>]), ".",)? stringify!($name)))
                .args(($($arg$(.cast($ty))?,)*))
        }
    )*}};
}

functions! {
    pub extern "pg" fn test_fn(v: pg::Type::TEXT) in TestSchema;
}
