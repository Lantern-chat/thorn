use super::name::{Name, Schema};

// WIP: Note sure what I'll do with this yet.
pub struct AnyTable {
    schema: Schema,
    name: Name,
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

pub trait Column: Collectable + 'static {
    fn name(&self) -> &'static str;
    fn ty(&self) -> pg::Type;
    fn comment(&self) -> &'static str;
}

pub trait Table: Clone + Copy + Column + Sized + 'static {
    const SCHEMA: Schema;
    const NAME: Name;
    const ALIAS: Option<&'static str>;
    const COMMENT: &'static str;

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

        impl $crate::table::RealTable for $table {
            const COLUMNS: &'static [Self] = &[$($table::$field_name,)*];
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

            fn comment(&self) -> &'static str {
                match *self {
                    $($table::$field_name => $crate::tables!(@DOC_START $(#[$($field_meta)*])*)),*
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
    /// This is a test table
    /// Testing
    #[derive(Debug)]
    pub struct TestTable as "tt" in TestSchema {
        /// Some identifier
        Id: Type::INT8,
        /// Username
        UserName: Type::TEXT,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ColumnExpr<C: Column> {
    col: C,
}

use crate::expr::*;

impl<C: Column> Expr for ColumnExpr<C> {}
impl<C: Column> ValueExpr for ColumnExpr<C> {}

impl<C: Column> Arguments for ColumnExpr<C> {
    fn to_vec(self) -> Vec<Box<dyn Expr>> {
        vec![Box::new(self)]
    }
}

impl<C: Column> Collectable for ColumnExpr<C> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        write!(w, "\"{}\"", self.col.name())
    }
}

pub trait ColumnExt: Column + Sized {
    fn as_name_only(self) -> ColumnExpr<Self> {
        ColumnExpr { col: self }
    }
}

impl<C: Column + Sized> ColumnExt for C {}

pub trait TableAlias: 'static {
    type T: Table;
    const NAME: &'static str;
}

pub struct Alias<A: TableAlias>(pub A::T);

impl<A: TableAlias> Copy for Alias<A> {}
impl<A: TableAlias> Clone for Alias<A> {
    fn clone(&self) -> Self {
        *self
    }
}

#[macro_export]
macro_rules! decl_alias {
    ($($vis:vis $dst:ident = $src:ty),+) => {
        paste::paste! {$(
            #[doc(hidden)]
            mod [<__private_ $dst:snake _impl>] {
                use super::*;

                pub struct $dst;
                impl $crate::table::TableAlias for $dst {
                    type T = $src;
                    const NAME: &'static str = stringify!([<$dst:snake>]);
                }

                pub type Inner = $crate::table::Alias<$dst>;
            }

            $vis use [<__private_ $dst:snake _impl>]::Inner as $dst;
        )*}
    };
}

impl<A: TableAlias> Column for Alias<A> {
    fn name(&self) -> &'static str {
        self.0.name()
    }
    fn ty(&self) -> pg::Type {
        self.0.ty()
    }
    fn comment(&self) -> &'static str {
        self.0.comment()
    }
}

impl<A: TableAlias> Table for Alias<A> {
    const SCHEMA: Schema = <A::T as Table>::SCHEMA;
    const NAME: Name = <A::T as Table>::NAME;
    const ALIAS: Option<&'static str> = Some(A::NAME);
    const COMMENT: &'static str = <A::T as Table>::COMMENT;
}

impl<A: TableAlias> Collectable for Alias<A> {
    fn collect(&self, w: &mut dyn Write, _t: &mut Collector) -> fmt::Result {
        write!(w, "\"{}\".\"{}\"", A::NAME, self.name())
    }
}

impl<A: TableAlias> Expr for Alias<A> {}
impl<A: TableAlias> ValueExpr for Alias<A> {}

impl<A: TableAlias> Arguments for Alias<A> {
    fn to_vec(self) -> Vec<Box<dyn Expr>> {
        vec![Box::new(self)]
    }
}

impl<A: TableAlias> Alias<A> {
    pub const fn col(col: A::T) -> Self {
        Alias(col)
    }
}

tables! {
    struct SchemaColumns as "columns" in InformationSchema {
        TableName: Type::TEXT,
        TableSchema: Type::TEXT,
        ColumnName: Type::TEXT,
        UdtName: Type::TEXT,
    }

    pub struct TableParameters {
        TableName: Type::TEXT,
        TableSchema: Type::TEXT,
        ColumnName: Type::TEXT,
        UdtName: Type::TEXT,
    }
}

pub trait RealTable: Table {
    const COLUMNS: &'static [Self];

    /// Generates a query that will attempt to verify the table schema for each column by
    /// cross-referencing with PostgreSQL's `information_schema.columns` table.
    ///
    /// Returns `[bool, text, text, text, text, text]` for `[matches, column_name, table_name, table_schema, expected_udt, found_udt]`
    ///
    /// If all of the first column are true, the database schema at least matches the Rust representation.
    fn verify() -> crate::query::SelectQuery {
        use crate::*;

        let column_names =
            Literal::Array(Self::COLUMNS.iter().map(|c| c.name().lit()).collect()).cast(Type::TEXT_ARRAY);

        let column_types = Literal::Array(
            Self::COLUMNS
                .iter()
                .map(|c| c.ty().name().to_owned().lit())
                .collect(),
        )
        .cast(Type::TEXT_ARRAY);

        let table_schema = match Self::SCHEMA {
            Schema::None => Literal::NULL,
            Schema::Named(schema) => Literal::TextStr(schema),
        };

        let table_params = TableParameters::as_query(
            Query::select()
                .expr(table_schema.alias_to(TableParameters::TableSchema))
                .expr(Self::NAME.name().lit().alias_to(TableParameters::TableName))
                .expr(Builtin::unnest((column_names,)).alias_to(TableParameters::ColumnName))
                .expr(Builtin::unnest((column_types,)).alias_to(TableParameters::UdtName)),
        );

        Query::select()
            .with(table_params.exclude())
            .from(
                TableParameters::left_join_table::<SchemaColumns>().on(SchemaColumns::TableName
                    .equals(TableParameters::TableName)
                    .and(SchemaColumns::ColumnName.equals(TableParameters::ColumnName))
                    .and(
                        SchemaColumns::TableSchema
                            .equals(TableParameters::TableSchema)
                            .or(TableParameters::TableSchema.is_null()),
                    )),
            )
            .expr(TableParameters::UdtName.equals(SchemaColumns::UdtName))
            .cols(&[
                TableParameters::ColumnName,
                TableParameters::TableName,
                TableParameters::TableSchema,
                TableParameters::UdtName,
            ])
            .col(SchemaColumns::UdtName)
    }
}
