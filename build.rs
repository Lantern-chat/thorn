use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(&std::env::var("OUT_DIR")?).join("keywords.rs");
    let mut file = BufWriter::new(File::create(path)?);

    println!("cargo:rerun-if-changed=keywords.txt");
    let src = include_str!("./keywords.txt");

    file.write_all(
        br#"
#[doc(hidden)]
#[macro_export]
macro_rules! __isql {
        ($out:expr; -- $($tt:tt)*) => { $out.write_str("$$")?; __isql!($out; $($tt)*); };

        ($out:expr; $lit:literal $($tt:tt)*) => { $out.write_literal($lit)?; __isql!($out; $($tt)*); };

        ($out:expr; @$table:ident::$column:ident $($tt:tt)*) => {
            std::write!($out, "\"{}\"", <$table as $crate::table::Column>::name(&$table::$column))?;
            __isql!($out; $($tt)*);
        };

        ($out:expr; $table:ident::$column:ident $($tt:tt)*) => {
            std::write!($out, "\"{}\".\"{}\"",
                <$table as $crate::Table>::NAME.name(),
                <$table as $crate::table::Column>::name(&$table::$column))?;
            __isql!($out; $($tt)*);
        };
"#,
    )?;

    for keyword in src.split_whitespace() {
        writeln!(
            file,
            r#"($out:expr; {keyword} $($tt:tt)*) => {{ $out.write_str("{keyword}")?; __isql!($out; $($tt)*); }};"#
        )?;
    }

    file.write_all(
        br##"
        ($out:expr; $table:ident $($tt:tt)*) => {
            $crate::query::from_item::__write_table::<$table>($out)?;
            __isql!($out; $($tt)*);
        };

        // parameters
        ($out:expr; #{$param:expr} $($tt:tt)*) => {
            std::write!($out, "${}", $param)?;
            __isql!($out; $($tt)*);
        };

        // casts
        ($out:expr; :: $param:ident $($tt:tt)*) => {
            std::write!($out, "::{}", stringify!($param))?;
            __isql!($out; $($tt)*);
        };

        // parenthesis and function calls
        ($out:expr; $(.$func:ident)? ( $($it:tt)* ) $($tt:tt)*) => {
            $($out.write_str(stringify!($func))?; )?
            $out.write_str("(")?;
            __isql!($out; $($it)*);
            $out.write_str(")")?;
            __isql!($out; $($tt)*);
        };

        // square brackets/array
        ($out:expr; [ $($it:tt)* ] $($tt:tt)*) => {
            $out.write_str("[")?;
            __isql!($out; $($it)*);
            $out.write_str("]")?;
            __isql!($out; $($tt)*);
        };

        // operators
        ($out:expr; && $($tt:tt)*) => { $out.write_str("AND")?; __isql!($out; $($tt)*); };
        ($out:expr; || $($tt:tt)*) => { $out.write_str("OR")?; __isql!($out; $($tt)*); };
    "##,
    )?;

    for op in OPERATORS {
        writeln!(
            file,
            r#"($out:expr; {op} $($tt:tt)*) => {{ $out.write_str("{op}")?; __isql!($out; $($tt)*); }};"#
        )?;
    }

    file.write_all(
        br##"
        // arbitrary runtime expressions
        ($out:expr; $value:block $($tt:tt)*) => {
            std::write!($out, "{}", { $value })?;
            __isql!($out; $($tt)*);
        };

        ($out:expr;) => {}
}"##,
    )?;

    Ok(())
}

const OPERATORS: &[&str] = &[
    "/||", "@@", "@>", "<@", "^@", "/|", "!!", "<<", ">>", "<>", "!=", ">=", "<=", ">", "<", "#", "~", "^",
    "|", "&", "%", "/", "*", "-", "+", "=", "!", ",", ";",
];
