/// Wrapper around `#[derive(ToSql, FromSql)]` that adds snake_case names and renaming
#[macro_export]
macro_rules! enums {
    ($( $(#[$meta:meta])* $enum_vis:vis enum $name:ident $(as $rename:tt)? {
        $(     $(#[$variant_meta:meta])* $variant:ident     ),+$(,)?
    })*) => {$crate::paste::paste! {$(
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, $crate::pg::ToSql, $crate::pg::FromSql)]
        #[postgres(name = "" [<$name:snake>] "")]
        $( #[postgres(name = "" [<$rename:snake>] "")] )?
        $enum_vis enum $name {$(
            $(#[$variant_meta])*
            #[postgres(name = "" [<$variant:snake>] "")]
            $variant
        ),*}
    )*}}
}

enums! {
    pub enum TestEnum as TestingEnum {
        Test
    }
}
