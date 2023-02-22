extern crate tokio_postgres as pgt;

use heck::*;

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{self, Write};

use pg::{Oid, Type};
use pgt::Error as PgError;

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
    name: &'a str,
    position: f32,
}

#[derive(Debug)]
struct Enum<'a> {
    name: &'a str,
    comment: Option<&'a str>,
    variants: Vec<Variant<'a>>,
    oid: Oid,
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

use crate::{table::SchemaColumns as Columns, *};

pub async fn generate(client: &pgt::Client, schema: Option<String>) -> Result<String, Error> {
    let table_oid = || {
        Call::custom("to_regclass")
            .args(Columns::TableSchema.concat(".".lit()).concat(Columns::TableName))
            .cast(Type::OID)
    };

    let query_columns = Query::select()
        .cols(&[
            /*0*/ Columns::TableName,
            /*1*/ Columns::ColumnName,
            /*2*/ Columns::UdtName,
            /*3*/ Columns::OrdinalPosition,
        ])
        // 4
        .expr(Columns::IsNullable.cast(Type::BOOL))
        // 5
        .col(PgType::Oid)
        // 6
        .expr(Call::custom("pg_catalog.obj_description").arg(table_oid()))
        // 7
        .expr(
            Call::custom("pg_catalog.col_description")
                .arg(table_oid())
                .arg(Columns::OrdinalPosition),
        )
        .from(Columns::left_join_table::<PgType>().on(PgType::Typname.equals(Columns::UdtName)))
        .and_where(Columns::TableSchema.equals(Var::of(Columns::TableSchema)))
        .order_by(Columns::TableName.ascending())
        .to_string()
        .0;

    let query_enums = Query::select()
        .from(
            PgEnum::inner_join_table::<PgType>()
                .on(PgType::Oid.equals(PgEnum::Enumtypid))
                .left_join_table::<PgNamespace>()
                .on(PgNamespace::Oid.equals(PgType::Typnamespace)),
        )
        .and_where(PgNamespace::Nspname.equals(Var::of(PgNamespace::Nspname)))
        .cols(&[PgType::Oid, PgType::Typname])
        .cols(&[PgEnum::Enumlabel, PgEnum::Enumsortorder])
        .expr(Call::custom("pg_catalog.obj_description").arg(PgType::Oid))
        .to_string()
        .0;

    let query_procs = Query::select()
        .from(
            PgProc::inner_join_table::<PgNamespace>()
                .on(PgNamespace::Oid.equals(PgProc::Pronamespace))
                .left_join_table::<PgDescription>()
                .on(PgDescription::Objoid.equals(PgProc::Oid)),
        )
        .and_where(PgNamespace::Nspname.equals(Var::of(PgNamespace::Nspname)))
        .cols(&[PgProc::Proname, PgProc::Proargnames, PgProc::Proargtypes])
        .col(PgDescription::Description)
        .and_where(PgProc::Provariadic.equals(0.lit()))
        .and_where(PgProc::Prorettype.not_equals((Type::TRIGGER.oid() as i32).lit()))
        .to_string()
        .0;

    let column_rows = client.query(query_columns.as_str(), &[&schema]).await?;
    let enum_variants = client.query(query_enums.as_str(), &[&schema]).await?;
    let proc_rows = client.query(query_procs.as_str(), &[&schema]).await?;

    let mut tables = HashMap::new();

    for row in &column_rows {
        let table_name: &str = row.try_get(0)?;
        let column_name: &str = row.try_get(1)?;
        let udt_name: &str = row.try_get(2)?;
        let nullable: bool = row.try_get(4)?;
        let position = row.try_get(3)?;
        let oid = row.try_get(5)?;
        let table_comment: Option<&str> = row.try_get(6)?;
        let col_comment: Option<&str> = row.try_get(7)?;

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

    let mut enums = HashMap::new();

    for row in &enum_variants {
        let enum_oid: Oid = row.try_get(0)?;
        let enum_name: &str = row.try_get(1)?;
        let enum_variant: &str = row.try_get(2)?;
        let enum_position: f32 = row.try_get(3)?;
        let enum_comment: Option<&str> = row.try_get(4)?;

        let enum_ = enums.entry(enum_name).or_insert_with(|| Enum {
            name: enum_name,
            comment: enum_comment,
            variants: Vec::new(),
            oid: enum_oid,
        });

        enum_.variants.push(Variant {
            name: enum_variant,
            position: enum_position,
        });
    }

    let mut enums = enums.into_values().collect::<Vec<_>>();

    let mut procs = Vec::new();
    for row in &proc_rows {
        let argnames: Option<Vec<&str>> = row.try_get(1)?;
        let argtypes: Vec<Oid> = row.try_get(2)?;

        // actual arguments names may not be present, so fill them with "__argN" names
        let argnames = match argnames {
            Some(argnames) => argnames
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    if name.is_empty() {
                        Cow::Owned(format!("__arg{i}"))
                    } else {
                        Cow::Borrowed(*name)
                    }
                })
                .collect(),
            None => argtypes
                .iter()
                .enumerate()
                .map(|(i, _)| Cow::Owned(format!("__arg{i}")))
                .collect(),
        };

        procs.push(Proc {
            name: row.try_get(0)?,
            argnames,
            argtypes,
            comment: row.try_get(3)?,
            rettype: 0,
        });
    }

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
                for line in textwrap::wrap(comment, 70) {
                    writeln!(out, "    /// {line}")?;
                }
            }

            write!(out, "    pub extern \"pg\" fn {}(", proc.name)?;

            for (idx, (arg, &ty)) in proc.argnames.iter().zip(&proc.argtypes).enumerate() {
                let ty = match Type::from_oid(ty) {
                    Some(ty) => Some(PType(ty).to_string()),
                    None => match enums.iter().find(|e| e.oid == ty) {
                        Some(enum_) => Some(format!("{}.clone()", enum_.name.to_shouty_snake_case())),
                        None => {
                            eprintln!("Cannot find type: {} for {}.{}", ty, proc.name, arg);
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
                Some(ref schema_name) => writeln!(out, ") in {schema_name};\n")?,
                None => writeln!(out, ");\n")?,
            }
        }

        out.push_str("}\n\n");
    }

    // Enums
    {
        let out = &mut out_enums;

        enums.sort_by_key(|e| e.name);

        let mut lazy_statics = String::new();
        lazy_statics.push_str("lazy_static::lazy_static! {\n");

        out.push_str("thorn::enums! {\n");

        for enum_ in &mut enums {
            if let Some(comment) = enum_.comment {
                for line in textwrap::wrap(comment, 70) {
                    writeln!(out, "    /// {line}")?;
                }
            }

            let enum_name = enum_.name.to_upper_camel_case();

            writeln!(
                lazy_statics,
                "    /// See [{enum_name}] for full documentation\n    pub static ref {}: Type = <{enum_name} as EnumType>::ty({});",
                enum_.name.to_shouty_snake_case(),
                enum_.oid
            )?;

            match schema_name {
                Some(ref name) => write!(out, "    pub enum {enum_name} in {name} {{\n")?,
                None => write!(out, "    pub enum {enum_name} {{\n")?,
            }

            enum_.variants.sort_by(|a, b| a.position.total_cmp(&b.position));

            for variant in &enum_.variants {
                let variant_name = variant.name.to_upper_camel_case();

                writeln!(out, "        {variant_name},")?;
            }

            out.push_str("    }\n");
        }

        lazy_statics.push_str("}\n\n");

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
                for line in textwrap::wrap(comment, 70) {
                    writeln!(out, "    /// {line}")?;
                }
            }

            let table_name = table.name.to_upper_camel_case();
            match schema_name {
                Some(ref name) => write!(out, "    pub struct {table_name} in {name} {{\n")?,
                None => write!(out, "    pub struct {table_name} {{\n")?,
            }

            table.cols.sort_by_key(|c| c.position);

            for col in &table.cols {
                let ty = match Type::from_oid(col.ty) {
                    Some(ty) => PType(ty).to_string(),
                    None => match enums.iter().find(|e| e.oid == col.ty) {
                        Some(enum_) => format!("{}.clone()", enum_.name.to_shouty_snake_case()),
                        None => {
                            eprintln!("Cannot find type: {} for {}.{}", col.udt, table.name, col.name);
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

    out += "use thorn::{enums::EnumType, pg::Type, table::Nullable};\n\n";

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
