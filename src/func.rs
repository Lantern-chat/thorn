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
            #[doc(hidden)]
            #[allow(unused)]
            pub const fn $name( $( $arg: (), )* ) {}
        }
    )*}};

    (@COUNT $arg:ident $($rest:ident)*) => {
        1 + $crate::functions!(@COUNT $($rest)*)
    };

    (@COUNT) => { 0 };
}

functions! {
    pub(crate) extern "pg" fn test_fn(v: pg::Type::TEXT, x: pg::Type::INT8) in TestSchema;
}
