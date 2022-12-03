use crate::Collectable;

use super::name::{Name, Schema};

pub trait EnumType: Collectable + Clone + Copy + Sized + 'static {
    const NAME: Name;
    const SCHEMA: Schema;

    const VARIANTS: &'static [Self];

    fn full_name() -> String {
        match Self::SCHEMA {
            Schema::None => format!("\"{}\"", Self::NAME.name()),
            Schema::Named(name) => format!("\"{}\".\"{}\"", name, Self::NAME.name()),
        }
    }

    /// Create a new [pg::Type] instance with the given oid value.
    fn ty(oid: u32) -> pg::Type;
}

/// Wrapper around `#[derive(ToSql, FromSql)]` that adds snake_case names and renaming
#[macro_export]
macro_rules! enums {
    ($( $(#[$meta:meta])* $enum_vis:vis enum $name:ident $(as $rename:tt)? $(in $schema:ident)? {
        $(     $(#[$variant_meta:meta])* $variant:ident     ),+$(,)?
    })*) => {$crate::paste::paste! {$(
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, $crate::pg::ToSql, $crate::pg::FromSql)]
        #[postgres(name = "" [<$name:snake>] "")]
        $( #[postgres(name = $rename:snake)] )?
        $enum_vis enum $name {$(
            $(#[$variant_meta])*
            #[postgres(name = "" [<$variant:snake>] "")]
            $variant
        ),*}

        impl $crate::enums::EnumType for $name {
            const SCHEMA: $crate::name::Schema = $crate::name::Schema::None
                $(.set(stringify!([<$schema:snake>])))?;

            const NAME: $crate::name::Name = $crate::name::Name::Default(stringify!([<$name:snake>])) $(.custom($rename))?;
            const VARIANTS: &'static [Self] = &[$($name::$variant),*];

            fn ty(oid: u32) -> pg::Type {
                pg::Type::new(
                    Self::NAME.name().to_owned(),
                    oid,
                    pg::Kind::Enum(vec![$( stringify!([<$variant:snake>]).to_owned() ),*]),
                    concat!("" $(, stringify!([<$schema:snake>]))?).to_owned()
                )
            }
        }

        impl $crate::collect::Collectable for $name {
            fn collect(&self, w: &mut dyn std::fmt::Write, _: &mut $crate::collect::Collector) -> std::fmt::Result {
                use $crate::enums::EnumType;

                write!(w, "'{}'::{}", match *self {
                    $( $name::$variant => stringify!([< $variant:snake >]) ),*
                }, Self::full_name())
            }
        }

        impl $crate::Expr for $name {}
        impl $crate::ValueExpr for $name {}

        impl $crate::Arguments for $name {
            fn to_vec(self) -> Vec<Box<dyn $crate::Expr>> {
                vec![Box::new(self)]
            }
        }
    )*}}
}

enums! {
    pub enum TestEnum as "testing_enum" in TestSchema {
        Test
    }
}
