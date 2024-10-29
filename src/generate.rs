use heck::*;
use name::Schema;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{self, Write};

use pg::{Oid, Type};
use pgt::Error as PgError;

use crate::extensions::{ClientExt, Error as ExtError};

crate::tables! {
    struct PgAttribute {
        Attrelid: Type::OID,
        Attname: Type::NAME,
    }

    struct PgType in PgCatalog {
        Oid: Type::OID,
        Typname: Type::NAME,
        Typelem: Type::OID,
        Typnamespace: Type::OID,
    }

    struct PgEnum in PgCatalog {
        Oid: Type::OID,
        Enumtypid: Type::OID,
        Enumsortorder: Type::FLOAT4,
        Enumlabel: Type::TEXT,
    }

    struct PgNamespace in PgCatalog {
        Oid: Type::OID,
        Nspname: Type::NAME,
    }

    struct PgDescription in PgCatalog {
        Objoid: Type::OID,
        Classoid: Type::OID,
        Description: Type::TEXT,
    }

    struct PgProc in PgCatalog {
        Oid: Type::OID,
        Proname: Type::NAME,
        Pronamespace: Type::OID,
        Provariadic: Type::OID,
        Prorettype: Type::OID,
        Proargtypes: Type::OID_VECTOR,
        Proargnames: Type::TEXT_ARRAY,
    }

}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    PgError(#[from] PgError),

    #[error(transparent)]
    Fmt(#[from] std::fmt::Error),

    #[error(transparent)]
    Ext(#[from] ExtError),
}

struct Proc<'a> {
    name: &'a str,
    argnames: Vec<Cow<'a, str>>,
    argtypes: Vec<Oid>,
    rettype: Oid,
    comment: Option<&'a str>,
}

#[derive(Debug)]
struct Variant<'a> {
    oid: Oid,
    name: &'a str,
    position: f32,
}

#[derive(Debug)]
struct Enum<'a> {
    oid: Oid,
    name: &'a str,
    comment: Option<&'a str>,
    variants: Vec<Variant<'a>>,
}

struct Column<'a> {
    name: &'a str,
    null: bool,
    udt: &'a str,
    ty: Oid,
    position: i32,
    comment: Option<&'a str>,
}

struct Table<'a> {
    comment: Option<&'a str>,
    name: &'a str,
    cols: Vec<Column<'a>>,
}

use crate::{table::SchemaColumns, *};

const COMMENT_WIDTH: usize = 70;

pub async fn generate(client: &pgt::Client, schema: Option<String>) -> Result<String, Error> {
    #[rustfmt::skip]
    let columns_rows = client.query2(sql! {
        const _: () = assert!(!Columns::IS_DYNAMIC);

        SELECT
            SchemaColumns.TableName AS @TableName,
            SchemaColumns.ColumnName AS @ColumnName,
            SchemaColumns.UdtName AS @UdtName,
            SchemaColumns.OrdinalPosition AS @Position,
            SchemaColumns.IsNullable::BOOL AS @Nullable,
            PgType.Oid AS @Oid,

            // pg_catalog.obj_description(to_regclass("columns"."table_schema" || '.' || "columns"."table_name")::oid)
            pg_catalog.obj_description(
                to_regclass(SchemaColumns.TableSchema || "." || SchemaColumns.TableName)::OID
            ) AS @TableComment,

            // pg_catalog.col_description(to_regclass("columns"."table_schema" || '.' || "columns"."table_name")::oid, "columns"."ordinal_position")
            pg_catalog.col_description(
                to_regclass(SchemaColumns.TableSchema || "." || SchemaColumns.TableName)::OID,
                SchemaColumns.OrdinalPosition
            ) AS @ColComment

        FROM SchemaColumns LEFT JOIN PgType ON PgType.Typname = SchemaColumns.UdtName
        WHERE SchemaColumns.TableSchema = #{&schema as SchemaColumns::TableSchema}
        ORDER BY SchemaColumns.TableName ASC
    })
    .await?;

    #[rustfmt::skip]
    let enums_rows = client.query2(sql! {
        const _: () = assert!(!Columns::IS_DYNAMIC);

        SELECT
            PgEnum.Enumtypid AS @Oid,
            PgEnum.Oid AS @VariantOid,
            PgType.Typname AS @Typname,
            PgEnum.Enumlabel AS @Enumlabel,
            PgEnum.Enumsortorder AS @Enumsortorder,
            pg_catalog.obj_description(PgType.Oid) AS @Description
        FROM PgEnum
            INNER JOIN PgType ON PgType.Oid = PgEnum.Enumtypid
            LEFT JOIN PgNamespace ON PgNamespace.Oid = PgType.Typnamespace
        WHERE PgNamespace.Nspname = #{&schema as PgNamespace::Nspname}
    }).await?;

    #[rustfmt::skip]
    let procs_rows = client.query2(sql! {
        const _: () = assert!(!Columns::IS_DYNAMIC);

        assert_eq!(Type::TRIGGER.oid(), 2279);

        SELECT
            PgProc.Proname AS @Proname,
            PgProc.Proargnames AS @Proargnames,
            PgProc.Proargtypes AS @Proargtypes,
            PgDescription.Description AS @Description
        FROM PgProc
        INNER JOIN PgNamespace ON PgNamespace.Oid = PgProc.Pronamespace
        LEFT JOIN PgDescription ON PgDescription.Objoid = PgProc.Oid
        WHERE PgNamespace.Nspname = #{&schema as PgNamespace::Nspname}
            AND PgProc.Provariadic = 0
            AND PgProc.Prorettype != const { 2279_i32 }
    }).await?;

    let mut tables = HashMap::new();
    let mut enums = HashMap::new();
    let mut procs = Vec::new();

    let mut uses_nullable = false;
    let mut uses_type = false;

    for row in &columns_rows {
        let table_name: &str = row.table_name()?;
        let column_name: &str = row.column_name()?;
        let udt_name: &str = row.udt_name()?;
        let nullable: bool = row.nullable()?;
        let position = row.position()?;
        let oid = row.oid()?;
        let table_comment: Option<&str> = row.table_comment()?;
        let col_comment: Option<&str> = row.col_comment()?;

        let table = tables.entry(table_name).or_insert_with(|| Table {
            name: table_name,
            comment: table_comment,
            cols: Vec::new(),
        });

        table.cols.push(Column {
            name: column_name,
            null: nullable,
            udt: udt_name,
            position,
            comment: col_comment,
            ty: oid,
        });
    }

    for row in &enums_rows {
        let oid: Oid = row.oid()?;
        let variant_oid: Oid = row.variant_oid()?;
        let enum_name: &str = row.typname()?;
        let enum_variant: &str = row.enumlabel()?;
        let enum_position: f32 = row.enumsortorder()?;
        let enum_comment: Option<&str> = row.description()?;

        let enum_ = enums.entry(enum_name).or_insert_with(|| Enum {
            oid,
            name: enum_name,
            comment: enum_comment,
            variants: Vec::new(),
        });

        enum_.variants.push(Variant {
            oid: variant_oid,
            name: enum_variant,
            position: enum_position,
        });
    }

    for row in &procs_rows {
        let argnames: Option<Vec<&str>> = row.proargnames()?;
        let argtypes: Vec<Oid> = row.proargtypes()?;

        // actual arguments names may not be present, so fill them with "__argN" names
        let argnames = match argnames {
            Some(argnames) => argnames
                .iter()
                .enumerate()
                .map(
                    |(i, name)| {
                        if name.is_empty() {
                            Cow::Owned(format!("__arg{i}"))
                        } else {
                            Cow::Borrowed(*name)
                        }
                    },
                )
                .collect(),
            None => argtypes.iter().enumerate().map(|(i, _)| Cow::Owned(format!("__arg{i}"))).collect(),
        };

        procs.push(Proc {
            name: row.proname()?,
            argnames,
            argtypes,
            comment: row.description()?,
            rettype: 0,
        });
    }

    // get enum values and ignore the keys
    let mut enums = enums.into_values().collect::<Vec<_>>();

    let schema_name = schema.map(|s| s.to_upper_camel_case());

    let mut out_funcs = String::new();
    let mut out_enums = String::new();
    let mut out_tables = String::new();

    // Funcs
    {
        let out = &mut out_funcs;

        procs.sort_by_key(|f| f.name);

        out.push_str("thorn::functions! {\n");

        for proc in procs {
            if let Some(comment) = proc.comment {
                for line in textwrap::wrap(comment, COMMENT_WIDTH) {
                    writeln!(out, "    /// {line}")?;
                }
            }

            write!(out, "    pub extern \"pg\" fn {}(", proc.name)?;

            for (idx, (arg, &ty)) in proc.argnames.iter().zip(&proc.argtypes).enumerate() {
                let ty = match Type::from_oid(ty) {
                    Some(ty) => {
                        uses_type = true;

                        Some(PType(ty).to_string())
                    }
                    None => match enums.iter().find(|e| e.oid == ty) {
                        Some(enum_) => Some(format!("{}.clone()", enum_.name.to_shouty_snake_case())),
                        None => {
                            eprintln!("Warning: Cannot find type: '{}' for '{}.{}'", ty, proc.name, arg);
                            None
                        }
                    },
                };

                match ty {
                    Some(ty) => write!(out, "{}: {}", arg, ty)?,
                    None => write!(out, "{}", arg)?,
                }

                if (idx + 1) < proc.argnames.len() {
                    out.push_str(", ");
                }
            }

            match schema_name {
                Some(ref schema_name) => writeln!(out, ") in {schema_name};")?,
                None => writeln!(out, ");")?,
            }
        }

        out.push_str("}\n\n");
    }

    // Enums
    {
        let out = &mut out_enums;

        enums.sort_by_key(|e| e.name);

        let mut lazy_statics = String::new();

        out.push_str("thorn::enums! {\n");

        for enum_ in &mut enums {
            if let Some(comment) = enum_.comment {
                for line in textwrap::wrap(comment, COMMENT_WIDTH) {
                    writeln!(out, "    /// {line}")?;
                }
            }

            let enum_name = enum_.name.to_upper_camel_case();

            uses_type = true;

            writeln!(
                lazy_statics,
                "/// See [{enum_name}] for full documentation\npub static {}: std::sync::LazyLock<Type> = std::sync::LazyLock::new(|| <{enum_name} as thorn::EnumType>::ty({}));\n",
                enum_.name.to_shouty_snake_case(),
                enum_.oid
            )?;

            // This isn't strictly necessary, but it's kind of a pointless rule for SQL enums
            out.push_str("    #[allow(clippy::enum_variant_names)]\n");

            match schema_name {
                Some(ref name) => writeln!(out, "    pub enum {enum_name} in {name} {{")?,
                None => writeln!(out, "    pub enum {enum_name} {{")?,
            }

            enum_.variants.sort_by(|a, b| a.position.total_cmp(&b.position));

            for variant in &enum_.variants {
                let variant_name = variant.name.to_upper_camel_case();

                writeln!(out, "        {variant_name},")?;
            }

            out.push_str("    }\n");
        }

        out.push_str("}\n\n");
        out.push_str(&lazy_statics);
    }

    // Tables
    {
        let out = &mut out_tables;

        out.push_str("thorn::tables! {\n");
        let mut tables = tables.into_values().collect::<Vec<_>>();
        tables.sort_by_key(|t| t.name);

        for mut table in tables {
            if let Some(comment) = table.comment {
                for line in textwrap::wrap(comment, COMMENT_WIDTH) {
                    writeln!(out, "    /// {line}")?;
                }
            }

            let table_name = table.name.to_upper_camel_case();
            match schema_name {
                Some(ref name) => writeln!(out, "    pub struct {table_name} in {name} {{")?,
                None => writeln!(out, "    pub struct {table_name} {{")?,
            }

            table.cols.sort_by_key(|c| c.position);

            for col in &table.cols {
                let ty = match Type::from_oid(col.ty) {
                    Some(ty) => {
                        uses_type = true;
                        PType(ty).to_string()
                    }
                    None => match enums.iter().find(|e| e.oid == col.ty) {
                        Some(enum_) => format!("{}.clone()", enum_.name.to_shouty_snake_case()),
                        None => {
                            eprintln!(
                                "Warning: Cannot find type: '{}' for '{}.{}'",
                                col.udt, table.name, col.name
                            );
                            continue;
                        }
                    },
                };

                let column_name = col.name.to_upper_camel_case();

                if let Some(comment) = col.comment {
                    for line in textwrap::wrap(comment, 66) {
                        writeln!(out, "        /// {line}")?;
                    }
                }

                if col.null {
                    uses_nullable = true;

                    writeln!(out, "        {column_name}: Nullable({}),", ty)?;
                } else {
                    writeln!(out, "        {column_name}: {},", ty)?;
                }
            }

            out.push_str("    }\n\n");
        }

        out.push_str("}\n");
    }

    let mut out = String::new();

    if uses_nullable {
        out.push_str("use thorn::table::Nullable;\n\n");
    }

    if uses_type {
        out.push_str("use thorn::pg::Type;\n\n");
    }

    out += &out_funcs;
    out += &out_enums;
    out += &out_tables;

    Ok(out)
}

struct PType(pub Type);

impl fmt::Display for PType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Type::{}", format!("{:?}", self.0).to_shouty_snake_case())
    }
}
