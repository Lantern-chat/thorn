use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(&std::env::var("OUT_DIR")?).join("sql_macro.rs");
    let mut file = BufWriter::new(File::create(path)?);

    println!("cargo:rerun-if-changed=keywords.txt");
    let src = include_str!("./keywords.txt");

    file.write_all(
        br#"
#[doc(hidden)]
#[macro_export]
macro_rules! __isql {
        ($out:expr; -- $($tt:tt)*) => { $out.inner().write_str("$$ ")?; __isql!($out; $($tt)*); };

        ($out:expr; $lit:literal $($tt:tt)*) => { $out.write_literal($lit)?; __isql!($out; $($tt)*); };

        ($out:expr; break; $($tt:tt)*) => {
            break;
        };

        ($out:expr; continue; $($tt:tt)*) => {
            continue;
        };

        ($out:expr; return; $($tt:tt)*) => {
            return Ok(());
        };

        ($out:expr; let $pat:pat_param = $expr:expr; $($tt:tt)*) => {
            let $pat = $expr;
            __isql!($out; $($tt)*);
        };

        ($out:expr; for $pat:pat in $it:expr; do { $($bt:tt)* } $($tt:tt)* ) => {
            for $pat in $it {
                __isql!($out; $($bt)*);
            }

            __isql!($out; $($tt)*);
        };

        ($out:expr; for $pat:pat in $it:expr; join $($join:literal)? { $($bt:tt)* } $($tt:tt)* ) => {
            let mut first = true;
            for $pat in $it {
                if !first {
                    $out.inner().write_str(($($join,)? ",",).0)?;
                }
                first = false;
                __isql!($out; $($bt)*);
            }

            __isql!($out; $($tt)*);
        };

        ($out:expr; if let $binding:pat = $value:expr; do { $($at:tt)* } else { $($bt:tt)* } $($tt:tt)* ) => {
            if let $binding = $value {
                __isql!($out; $($at)*);
            } else {
                __isql!($out; $($bt)*);
            }
            __isql!($out; $($tt)*);
        };

        ($out:expr; if let $binding:pat = $value:expr; do { $($at:tt)* } $($tt:tt)* ) => {
            if let $binding = $value {
                __isql!($out; $($at)*);
            }
            __isql!($out; $($tt)*);
        };

        ($out:expr; if $cond:expr; do { $($at:tt)* } else { $($bt:tt)* } $($tt:tt)* ) => {
            if $cond {
                __isql!($out; $($at)*);
            } else {
                __isql!($out; $($bt)*);
            }
            __isql!($out; $($tt)*);
        };

        ($out:expr; if $cond:expr; do { $($at:tt)* } $($tt:tt)* ) => {
            if $cond {
                __isql!($out; $($at)*);
            }
            __isql!($out; $($tt)*);
        };

        ($out:expr; match $cond:expr; do {$(
            $pat:pat $(if $pat_cond:expr)? => { $($pt:tt)* } $(,)?
        )*} $($tt:tt)*) => {
            match $cond {$(
                $pat $(if $pat_cond)? => {
                    __isql!($out; $($pt)*);
                },
            )*}

            __isql!($out; $($tt)*);
        };

        ($out:expr; AS $table:ident.$column:ident $($tt:tt)*) => {
            std::write!($out.inner(), "AS \"{}\" ", <$table as $crate::table::Column>::name(&$table::$column))?;
            __isql!($out; $($tt)*);
        };

        ($out:expr; $table:ident.$column:ident $($tt:tt)*) => {
            std::write!($out.inner(), "\"{}\".\"{}\" ",
                <$table as $crate::Table>::NAME.name(),
                <$table as $crate::table::Column>::name(&$table::$column))?;
            __isql!($out; $($tt)*);
        };

        ($out:expr; $var:ident++; $($tt:tt)*) => {
            $var += 1;
            __isql!($out; $($tt)*);
        };
        ($out:expr; $var:ident--; $($tt:tt)*) => {
            $var -= 1;
            __isql!($out; $($tt)*);
        };
"#,
    )?;

    for keyword in src.split_whitespace() {
        writeln!(
            file,
            r#"($out:expr; {keyword} $($tt:tt)*) => {{ $out.inner().write_str("{keyword} ")?; __isql!($out; $($tt)*); }};"#
        )?;
    }

    file.write_all(
        br##"
        ($out:expr; $table:ident $($tt:tt)*) => {
            $crate::query::from_item::__write_table::<$table>(&mut $out)?;
            __isql!($out; $($tt)*);
        };

        // parameters
        ($out:expr; #{$param:expr $(=> $ty:expr)?} $($tt:tt)*) => {
            {
                let param = $param;
                $out.param(param, ($($ty.into(),)? $crate::pg::Type::ANY,).0)?;
                std::write!($out, "${param}")?;
            }
            __isql!($out; $($tt)*);
        };

        // casts
        ($out:expr; :: $param:ident $($tt:tt)*) => {
            std::write!($out.inner(), "::{} ", stringify!($param))?;
            __isql!($out; $($tt)*);
        };

        ($out:expr; () $($tt:tt)*) => {
            $out.inner().write_str("() ")?;
            __isql!($out; $($tt)*);
        };

        ($out:expr; [] $($tt:tt)*) => {
            $out.inner().write_str("[] ")?;
            __isql!($out; $($tt)*);
        };

        // parenthesis and function calls
        ($out:expr; $(.$func:ident)? ( $($it:tt)* ) $($tt:tt)*) => {
            $out.inner().write_str(concat!( $(stringify!($func),)? "( "))?;
            __isql!($out; $($it)*);
            $out.inner().write_str(") ")?;
            __isql!($out; $($tt)*);
        };

        // arbitrary runtime function calls
        ($out:expr; .{$func:expr} ( $($it:tt)* ) $($tt:tt)*) => {
            write!($out.inner(), "{}( ", $func)?;
            __isql!($out; $($it)*);
            $out.inner().write_str(") ")?;
            __isql!($out; $($tt)*);
        };

        // square brackets/array
        ($out:expr; [ $($it:tt)* ] $($tt:tt)*) => {
            $out.inner().write_str("[ ")?;
            __isql!($out; $($it)*);
            $out.inner().write_str("] ")?;
            __isql!($out; $($tt)*);
        };

        // arbitrary runtime expressions
        ($out:expr; @$value:block $($tt:tt)*) => {
            std::write!($out, "{}", $value)?;
            __isql!($out; $($tt)*);
        };

        // arbitrary runtime type casting
        ($out:expr; ::$value:block $($tt:tt)*) => {
            std::write!($out, "::{}", $crate::pg::Type::from($value))?;
            __isql!($out; $($tt)*);
        };

        ($out:expr; !$block:block $($tt:tt)*) => {
            $block;
            __isql!($out; $($tt)*);
        };
    "##,
    )?;

    for token in TOKENS {
        writeln!(
            file,
            r#"($out:expr; {token} $($tt:tt)*) => {{ $out.inner().write_str("{token} ")?; __isql!($out; $($tt)*); }};"#
        )?;
    }

    file.write_all(
        br##"
        // arbitrary runtime literals
        ($out:expr; $value:block $($tt:tt)*) => {
            $out.write_literal($value)?;
            __isql!($out; $($tt)*);
        };

        ($out:expr;) => {}
}"##,
    )?;

    Ok(())
}

const TOKENS: &[&str] = &[
    "->>", "#>>", "/||", "@@", "@>", "<@", "^@", "/|", "&&", "||", "()", "[]", "!!", "->", "#>", "<<", ">>", "<>", "!=", ">=", "<=", ">", "<", "#",
    "~", "^", "|", "&", "%", "/", "*", "-", "+", "=", "!", ",", ";",
];
