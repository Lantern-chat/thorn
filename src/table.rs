pub enum Schema {
    None,
    Named(&'static str),
}

impl Schema {
    #[doc(hidden)]
    pub const fn set(self, name: &'static str) -> Self {
        Schema::Named(name)
    }
}

pub trait Table: Sized + 'static {
    const SCHEMA: Schema;
    const NAME: &'static str;
    const COLUMNS: &'static [Self];

    fn name(&self) -> &'static str;
    fn ty(&self) -> pg::Type;
}

#[macro_export]
macro_rules! table {
    ($(#[$meta:meta])* $struct_vis:vis enum $table:ident $(in $schema:ident)? {$(
        $field_name:ident: $ty:expr
    ),*$(,)?}) => {
        $(#[$meta])*
        $struct_vis enum $table {
            $($field_name,)*
        }

        impl $crate::Table for $table {
            const SCHEMA: $crate::table::Schema = $crate::table::Schema::None
                $(.set(paste::paste!(stringify!([<$schema:snake>]))))?;

            const NAME: &'static str = paste::paste!(stringify!([<$table:snake>]));
            const COLUMNS: &'static [Self] = &[$($table::$field_name),*];

            #[inline]
            fn name(&self) -> &'static str {
                match *self {
                    $($table::$field_name => paste::paste!(stringify!([<$field_name:snake>]))),*
                }
            }

            #[inline]
            fn ty(&self) -> $crate::pg::Type {
                match *self {
                    $($table::$field_name => $ty),*
                }
            }
        }

        impl $crate::collect::Collectable for $table {
            fn collect(&self, w: &mut dyn std::fmt::Write, _: &mut $crate::collect::Collector) -> std::fmt::Result {
                write!(w, "\"{}\".\"{}\"", Self::NAME, self.name())
            }
        }

        impl $crate::Expr for $table {}
    }
}

use pg::Type;

table! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum TestTable in TestSchema {
        Id: Type::INT8,
        UserName: Type::VARCHAR,
    }
}
