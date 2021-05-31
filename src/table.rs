#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Schema {
    None,
    Named(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TableName {
    Default(&'static str),
    Custom(&'static str),
}

impl Schema {
    #[doc(hidden)]
    pub const fn set(self, name: &'static str) -> Self {
        Schema::Named(name)
    }
}

impl TableName {
    #[doc(hidden)]
    pub const fn custom(self, name: &'static str) -> Self {
        TableName::Custom(name)
    }

    pub const fn name(&self) -> &'static str {
        match *self {
            TableName::Default(name) => name,
            TableName::Custom(name) => name,
        }
    }
}

// WIP: Note sure what I'll do with this yet.
pub struct AnyTable {
    schema: Schema,
    name: TableName,
    //columns: Vec<&'static dyn Column>,
}

use crate::collect::{Collectable, Collector};
use std::fmt::{self, Write};

impl Collectable for AnyTable {
    fn collect(&self, w: &mut dyn Write, _: &mut Collector) -> fmt::Result {
        match self.schema {
            Schema::None => write!(w, "\"{}\"", self.name.name()),
            Schema::Named(schema) => write!(w, "\"{}\".\"{}\"", schema, self.name.name()),
        }
    }
}

const _: Option<&dyn Column> = None;

pub trait Column: Collectable {
    fn name(&self) -> &'static str;
    fn ty(&self) -> pg::Type;
}

pub trait Table: Clone + Copy + Column + Sized + 'static {
    const SCHEMA: Schema;
    const NAME: TableName;
    const COLUMNS: &'static [Self];

    fn to_any() -> AnyTable {
        AnyTable {
            schema: Self::SCHEMA,
            name: Self::NAME,
            //columns: Self::COLUMNS.iter().map(|c| c as _).collect(),
        }
    }
}

#[macro_export]
macro_rules! tables {
    ($($(#[$meta:meta])* $struct_vis:vis struct $table:ident $(as $rename:tt)? $(in $schema:ident)? {$(
        $(#[$field_meta:meta])* $field_name:ident: $ty:expr
    ),*$(,)?})*) => {$crate::paste::paste! {$(
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        $struct_vis enum $table {
            $($(#[$field_meta])* $field_name,)*
        }

        impl $crate::Table for $table {
            const SCHEMA: $crate::table::Schema = $crate::table::Schema::None
                $(.set(stringify!([<$schema:snake>])))?;

            const NAME: $crate::table::TableName = $crate::table::TableName::Default(stringify!([<$table:snake>])) $(.custom($rename))?;
            const COLUMNS: &'static [Self] = &[$($table::$field_name),*];
        }

        impl $crate::table::Column for $table {
            #[inline]
            fn name(&self) -> &'static str {
                match *self {
                    $($table::$field_name => stringify!([<$field_name:snake>])),*
                }
            }

            #[inline]
            fn ty(&self) -> $crate::pg::Type {
                match *self {
                    $($table::$field_name => $crate::pg::Type::from($ty)),*
                }
            }
        }

        impl From<$table> for $crate::pg::Type {
            #[inline]
            fn from(t: $table) -> Self {
                use $crate::table::{Table, Column};

                t.ty()
            }
        }

        impl $crate::collect::Collectable for $table {
            fn collect(&self, w: &mut dyn std::fmt::Write, _: &mut $crate::collect::Collector) -> std::fmt::Result {
                use $crate::table::{Table, Column};

                write!(w, "\"{}\".\"{}\"", Self::NAME.name(), self.name())
            }
        }

        impl $crate::Expr for $table {}
        impl $crate::ValueExpr for $table {}

        impl $crate::Arguments for $table {
            fn to_vec(self) -> Vec<Box<dyn $crate::Expr>> {
                vec![Box::new(self)]
            }
        }
    )*}}
}

use pg::Type;

tables! {
    #[derive(Debug)]
    pub struct TestTable as "tt" in TestSchema {
        Id: Type::INT8,
        UserName: Type::VARCHAR,
    }
}
