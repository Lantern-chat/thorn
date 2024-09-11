use super::name::{Name, Schema};

pub struct ColumnType {
    pub pg: pg::Type,
    pub nullable: bool,
}

#[repr(transparent)]
pub struct Nullable<T>(pub T);

impl<T> From<Nullable<T>> for ColumnType
where
    T: Into<ColumnType>,
{
    fn from(ty: Nullable<T>) -> Self {
        let mut ty = ty.0.into();
        ty.nullable = true;
        ty
    }
}

impl From<pg::Type> for ColumnType {
    fn from(pg: pg::Type) -> Self {
        ColumnType { pg, nullable: false }
    }
}

use std::fmt::{self, Write};

const _: Option<&dyn Column> = None;

pub trait Column: 'static {
    fn name(&self) -> &'static str;
    fn ty(&self) -> ColumnType;
    fn comment(&self) -> &'static str;
}

pub trait Table: Clone + Copy + Column + Sized + 'static {
    const SCHEMA: Schema;
    const NAME: Name;
    const ALIAS: Option<&'static str>;
    const COMMENT: &'static str;
}

pub trait TableExt: Table {
    const TYPENAME: &'static str;
    const TYPENAME_SNAKE: &'static str;
}

#[macro_export]
macro_rules! tables {
    (@DOC #[doc = $doc:literal]) => { concat!($doc, "\n") };
    (@DOC #[$meta:meta]) => {""};

    (@DOC_START $(# $meta:tt)*) => {
        concat!($($crate::tables!(@DOC # $meta)),*)
    };

    ($($(#[$($meta:tt)*])* $struct_vis:vis struct $table:ident $(as $rename:tt)? $(in $schema:ident)? {$(
        $(#[$($field_meta:tt)*])* $field_name:ident: $ty:expr
    ),*$(,)?})*) => {$crate::paste::paste! {$(
        $(#[$($meta)*])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        $struct_vis enum $table {
            $($(#[$($field_meta)*])* $field_name,)*
        }

        impl $crate::Table for $table {
            const SCHEMA: $crate::name::Schema = $crate::name::Schema::None
                $(.set(stringify!([<$schema:snake>])))?;

            const NAME: $crate::name::Name = $crate::name::Name::Default(stringify!([<$table:snake>])) $(.custom($rename))?;
            const ALIAS: Option<&'static str> = None;
            const COMMENT: &'static str = $crate::tables!(@DOC_START $(#[$($meta)*])*);
        }

        impl $crate::table::TableExt for $table {
            const TYPENAME: &'static str = stringify!($table);
            const TYPENAME_SNAKE: &'static str = stringify!([<$table:snake>]);
        }

        // impl $crate::table::RealTable for $table {
        //     const COLUMNS: &'static [Self] = &[$($table::$field_name,)*];
        // }

        impl $crate::table::Column for $table {
            #[inline]
            fn name(&self) -> &'static str {
                match *self {
                    $($table::$field_name => stringify!([<$field_name:snake>])),*
                }
            }

            #[inline]
            fn ty(&self) -> $crate::table::ColumnType {
                match *self {
                    $($table::$field_name => $crate::table::ColumnType::from($ty)),*
                }
            }

            fn comment(&self) -> &'static str {
                match *self {
                    $($table::$field_name => $crate::tables!(@DOC_START $(#[$($field_meta)*])*)),*
                }
            }
        }

        impl From<$table> for $crate::table::ColumnType {
            #[inline]
            fn from(t: $table) -> Self {
                use $crate::table::{Table, Column};

                t.ty()
            }
        }

        impl From<$table> for $crate::pg::Type {
            #[inline]
            fn from(t: $table) -> Self {
                use $crate::table::{Table, Column};

                t.ty().pg
            }
        }
    )*}}
}

use pg::Type;

tables! {
    /// This is a test table
    /// Testing
    #[derive(Debug)]
    pub struct TestTable as "tt" in TestSchema {
        /// Some identifier
        Id: Type::INT8,
        /// Username
        UserName: Type::TEXT,
    }

    pub(crate) struct SchemaColumns as "columns" in InformationSchema {
        TableName: Type::NAME,
        TableSchema: Type::NAME,
        ColumnName: Type::NAME,
        UdtName: Type::NAME,
        UdtSchema: Type::NAME,
        IsNullable: Type::BOOL,
        OrdinalPosition: Type::INT4,
    }

    pub struct TableParameters {
        TableName: Type::NAME,
        TableSchema: Type::NAME,
        ColumnName: Type::NAME,
        UdtName: Type::NAME,
        IsNullable: Type::BOOL,
    }
}

// pub trait RealTable: Table {
//     const COLUMNS: &'static [Self];

//     /// Generates a query that will attempt to verify the table schema for each column by
//     /// cross-referencing with PostgreSQL's `information_schema.columns` table.
//     ///
//     /// Returns
//     /// ```ignore
//     /// [
//     ///     matches: bool,
//     ///     column_name: text,
//     ///     table_name: text,
//     ///     table_schema: text,
//     ///     expected_udt: text,
//     ///     expected_nullable: bool,
//     ///     found_udt: text,
//     ///     found_nullable: bool
//     /// ]
//     /// ```
//     ///
//     /// If all of the first column are true, the database schema at least matches the Rust representation.
//     fn verify() -> crate::query::SelectQuery {
//         use crate::*;

//         let column_names = Self::COLUMNS.iter().map(|c| c.name()).collect::<Vec<_>>().lit().cast(Type::TEXT_ARRAY);

//         let column_types = Self::COLUMNS
//             .iter()
//             .map(|c| c.ty().pg.name().to_owned())
//             .collect::<Vec<_>>()
//             .lit()
//             .cast(Type::TEXT_ARRAY);

//         let column_nullable =
//             Self::COLUMNS.iter().map(|c| c.ty().nullable).collect::<Vec<_>>().lit().cast(Type::BOOL_ARRAY);

//         let table_schema: Box<dyn ValueExpr> = match Self::SCHEMA {
//             Schema::None => Box::new(().lit()),
//             Schema::Named(schema) => Box::new(schema.lit()),
//         };

//         let table_params = TableParameters::as_query(
//             Query::select()
//                 .expr(table_schema.alias_to(TableParameters::TableSchema))
//                 .expr(Self::NAME.name().lit().alias_to(TableParameters::TableName))
//                 .expr(Builtin::unnest((column_names,)).alias_to(TableParameters::ColumnName))
//                 .expr(Builtin::unnest((column_types,)).alias_to(TableParameters::UdtName))
//                 .expr(Builtin::unnest((column_nullable,)).alias_to(TableParameters::IsNullable)),
//         );

//         Query::select()
//             .with(table_params.exclude())
//             .from(
//                 TableParameters::left_join_table::<SchemaColumns>().on(SchemaColumns::TableName
//                     .equals(TableParameters::TableName)
//                     .and(SchemaColumns::ColumnName.equals(TableParameters::ColumnName))
//                     .and(
//                         SchemaColumns::TableSchema
//                             .equals(TableParameters::TableSchema)
//                             .or(TableParameters::TableSchema.is_null()),
//                     )),
//             )
//             .expr(
//                 TableParameters::UdtName
//                     .equals(SchemaColumns::UdtName)
//                     .and(TableParameters::IsNullable.equals(SchemaColumns::IsNullable)),
//             )
//             .cols(&[
//                 TableParameters::ColumnName,
//                 TableParameters::TableName,
//                 TableParameters::TableSchema,
//                 TableParameters::UdtName,
//                 TableParameters::IsNullable,
//             ])
//             .cols(&[SchemaColumns::UdtName, SchemaColumns::IsNullable])
//     }
// }
