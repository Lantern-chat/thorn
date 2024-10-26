use super::name::{Name, Schema};

pub trait EnumType: Clone + Copy + Sized + 'static {
    const NAME: Name;
    const SCHEMA: Schema;

    const VARIANTS: &'static [Self];

    fn full_name() -> String {
        match Self::SCHEMA {
            Schema::None => format!("\"{}\"", Self::NAME.name()),
            Schema::Named(name) => format!("\"{}\".\"{}\"", name, Self::NAME.name()),
        }
    }

    fn name(&self) -> &'static str;

    /// Create a new [pg::Type] instance with the given oid value.
    fn ty(oid: u32) -> pg::Type {
        pg::Type::new(
            Self::NAME.name().to_owned(),
            oid,
            pg::Kind::Enum(Self::VARIANTS.iter().map(|v| v.name().to_owned()).collect()),
            match Self::SCHEMA {
                Schema::Named(name) => name.to_owned(),
                Schema::None => String::new(),
            },
        )
    }
}

/// Wrapper around `#[derive(ToSql, FromSql)]` that adds snake_case names and renaming
#[macro_export]
macro_rules! enums {
    ($( $(#[$meta:meta])* $enum_vis:vis enum $name:ident $(as $rename:tt)? $(in $schema:ident)? {
        $(     $(#[$variant_meta:meta])* $variant:ident     ),+$(,)?
    })*) => {$crate::paste::paste! {$(
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $enum_vis enum $name { $( $(#[$variant_meta])* $variant ),* }

        const _: () = {
            use std::error::Error;
            use $crate::pg::{ToSql, to_sql_checked, FromSql, IsNull, Type, Kind};
            use $crate::pg::private::BytesMut;
            use $crate::enums::EnumType;

            fn accepts(ty: &Type) -> bool {
                if ty.name() != $name::NAME.name() {
                    return false;
                }

                match *ty.kind() {
                    Kind::Enum(ref variants) if variants.len() == $name::VARIANTS.len() => {
                        variants.iter().all(|v| $name::VARIANTS.iter().any(|e| e.name() == v))
                    }
                    _ => false,
                }
            }

            impl ToSql for $name {
                fn to_sql(&self, _ty: &Type, buf: &mut BytesMut) -> std::result::Result<IsNull, Box<dyn Error + Sync + Send>> {
                    buf.extend_from_slice(self.name().as_bytes());
                    Ok(IsNull::No)
                }

                #[inline]
                fn accepts(ty: &Type) -> bool {
                    accepts(ty)
                }

                to_sql_checked!();
            }

            impl<'a> FromSql<'a> for $name {
                fn from_sql(_ty: &Type, buf: &'a [u8]) -> Result<$name, Box<dyn Error + Sync + Send>> {
                    match ::core::str::from_utf8(buf)? {
                        $(stringify!([<$variant:snake>]) => Ok($name::$variant),)*
                        s => Err(format!("Invalid variant: {s}").into())
                    }
                }

                #[inline]
                fn accepts(ty: &Type) -> bool {
                    accepts(ty)
                }
            }
        };

        impl $crate::enums::EnumType for $name {
            const SCHEMA: $crate::name::Schema = $crate::name::Schema::None
                $(.set(stringify!([<$schema:snake>])))?;

            const NAME: $crate::name::Name = $crate::name::Name::Default(stringify!([<$name:snake>])) $(.custom($rename))?;
            const VARIANTS: &'static [Self] = &[$($name::$variant),*];

            fn name(&self) -> &'static str {
                match *self {
                    $($name::$variant => stringify!([<$variant:snake>])),*
                }
            }
        }
    )*}}
}

enums! {
    #[allow(clippy::enum_variant_names)]
    pub enum TestEnum as "testing_enum" in TestSchema {
        Test
    }
}
