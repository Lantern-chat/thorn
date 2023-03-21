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

        ($out:expr; const $name:ident: $ty:ty = $expr:expr; $($tt:tt)*) => {
            const $name: $ty = $expr;
            __isql!($out; $($tt)*);
        };

        ($out:expr; for-join $({$join:literal})? $pat:pat in $($rest:tt)*) => {
            __isql!(@JOIN $out;
                pat = ($pat)
                join = ($($join)?)
                iter = ()
                rest = ($($rest)*)
            );
        };

        ($out:expr; for $pat:pat in $($rest:tt)*) => {
            __isql!(@FOR $out;
                pat = ($pat)
                iter = ()
                rest = ($($rest)*)
            );
        };

        ($out:expr; if $($rest:tt)*) => {
            __isql!(@BRANCH $out;
                pred = ()
                rest = ($($rest)*)
            );
        };

        ($out:expr; match $($rest:tt)*) => {
            __isql!(@MATCH $out;
                pred = ()
                rest = ($($rest)*)
            );
        };

        (@FOR $out:expr;
            pat = ($pat:pat)
            iter = ($($iter:tt)*)
            rest = ({ $($rest:tt)* } $($tt:tt)*)
        ) => {
            for $pat in $($iter)* {
                __isql!($out; $($rest)*);
            }

            __isql!($out; $($tt)*);
        };

        (@FOR $out:expr;
            pat = ($pat:pat)
            iter = ($($iter:tt)*)
            rest = ($next:tt $($rest:tt)*)
        ) => {
            __isql!(@FOR $out;
                pat = ($pat)
                iter = ($($iter)* $next)
                rest = ($($rest)*)
            );
        };

        (@JOIN $out:expr;
            pat = ($pat:pat)
            join = ($($join:literal)?)
            iter = ($($iter:tt)*)
            rest = ({ $($rest:tt)* } $($tt:tt)*)
        ) => {
            let mut first = true;
            for $pat in $($iter)* {
                if !first {
                    $out.inner().write_str(($($join,)? ",",).0)?;
                }
                first = false;

                __isql!($out; $($rest)*);
            }

            __isql!($out; $($tt)*);
        };

        (@JOIN $out:expr;
            pat = ($pat:pat)
            join = ($($join:literal)?)
            iter = ($($iter:tt)*)
            rest = ($next:tt $($rest:tt)*)
        ) => {
            __isql!(@JOIN $out;
                pat = ($pat)
                join = ($($join)?)
                iter = ($($iter)* $next)
                rest = ($($rest)*)
            );
        };

        // (@BRANCH $out:expr;
        //     pred = ( $($pred:tt)* )
        //     rest = ( { $($then:tt)* } else if $($rest:tt)* )
        // ) => {
        //     if $($pred)* {
        //         __isql!($out; $($then)*);
        //     } else {
        //         __isql!(@BRANCH $out;
        //             pred = ()
        //             rest = ($($rest)*)
        //         );
        //     }
        //     __isql!($out; $($tt)*);
        // };

        (@BRANCH $out:expr;
            pred = ( $($pred:tt)* )
            rest = ( { $($then:tt)* } else { $($else:tt)* } $($tt:tt)* )
        ) => {
            if $($pred)* {
                __isql!($out; $($then)*);
            } else {
                __isql!($out; $($else)*);
            }
            __isql!($out; $($tt)*);
        };

        (@BRANCH $out:expr;
            pred = ( $($pred:tt)* )
            rest = ( { $($then:tt)* } $($tt:tt)* )
        ) => {
            if $($pred)* {
                __isql!($out; $($then)*);
            }
            __isql!($out; $($tt)*);
        };

        (@BRANCH $out:expr;
            pred = ($($pred:tt)*)
            rest = ($next:tt $($rest:tt)*)
        ) => {
            __isql!(@BRANCH $out;
                pred = ($($pred)* $next)
                rest = ($($rest)*)
            );
        };

        (@MATCH $out:expr;
            pred = ($($pred:tt)*)
            rest = ( {
                $($pat:pat $(if $pat_cond:expr)? => { $($pt:tt)* } $(,)?)*
            } $($tt:tt)* )
        ) => {
            match $($pred)* {$(
                $pat $(if $pat_cond)? => {
                    __isql!($out; $($pt)*);
                },
            )*}

            __isql!($out; $($tt)*);
        };

        (@MATCH $out:expr;
            pred = ($($pred:tt)*)
            rest = ($next:tt $($rest:tt)*)
        ) => {
            __isql!(@MATCH $out;
                pred = ($($pred)* $next)
                rest = ($($rest)*)
            );
        };

        ($out:expr; AS $table:ident.$column:ident $($tt:tt)*) => {
            std::write!($out.inner(), "AS \"{}\" ", <$table as $crate::table::Column>::name(&$table::$column))?;
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
        ($out:expr; $table:ident.$column:ident $($tt:tt)*) => {
            $out.write_column($table::$column)?;
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

        ($out:expr; $table:ident $($tt:tt)*) => {
            $out.write_table::<$table>()?;
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
    "->>", "#>>", "/||", "@@", "@>", "<@", "^@", "/|", "&&", "||", "()", "[]", "!!", "->", "#>", "<<", ">>",
    "<>", "!=", ">=", "<=", ">", "<", "#", "~", "^", "|", "&", "%", "/", "*", "-", "+", "=", "!", ",", ";",
];
