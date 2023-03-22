use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

// https://stackoverflow.com/questions/51932944/how-to-match-rusts-if-expressions-in-a-macro

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
        ([$($stack:literal)*] $out:expr; -- $($tt:tt)*) => { __isql!([$($stack)* "$$"] $out; $($tt)*); };
        ([$($stack:literal)*] $out:expr; 1 $($tt:tt)*) => { __isql!([$($stack)* "1"] $out; $($tt)*); };

        (@FLUSH $out:expr; [$($stack:literal)+]) => {
            $out.inner().write_str(concat!($($stack, " ",)*))?;
        };

        (@FLUSH $out:expr; []) => {};

        ([$($stack:literal)*] $out:expr; $lit:literal $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.write_literal($lit)?; __isql!([] $out; $($tt)*);
        };

        ([$($stack:literal)*] $out:expr; break; $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            break;
        };

        ([$($stack:literal)*] $out:expr; continue; $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            continue;
        };

        ([$($stack:literal)*] $out:expr; return; $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            return Ok(());
        };

        ([$($stack:literal)*] $out:expr; let $pat:pat_param = $expr:expr; $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            let $pat = $expr;
            __isql!([] $out; $($tt)*);
        };

        ([$($stack:literal)*] $out:expr; const $name:ident: $ty:ty = $expr:expr; $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            const $name: $ty = $expr;
            __isql!([] $out; $($tt)*);
        };

        ([$($stack:literal)*] $out:expr; for-join $({$join:literal})? $pat:pat in $($rest:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            __isql!(@JOIN $out;
                pat = ($pat)
                join = ($($join)?)
                iter = ()
                rest = ($($rest)*)
            );
        };

        ([$($stack:literal)*] $out:expr; for $pat:pat in $($rest:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            __isql!(@FOR $out;
                pat = ($pat)
                iter = ()
                rest = ($($rest)*)
            );
        };

        ([$($stack:literal)*] $out:expr; if $($rest:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            __isql!(@BRANCH $out;
                pred = ()
                rest = ($($rest)*)
            );
        };

        ([$($stack:literal)*] $out:expr; match $($rest:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            __isql!(@MATCH $out;
                pred = ()
                rest = ($($rest)*)
            );
        };

        (@FOR $out:expr;
            pat = ($pat:pat)
            iter = ($($iter:tt)+)
            rest = ({ $($rest:tt)* } $($tt:tt)*)
        ) => {
            for $pat in $($iter)* {
                __isql!([] $out; $($rest)*);
            }

            __isql!([] $out; $($tt)*);
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
            iter = ($($iter:tt)+)
            rest = ({ $($rest:tt)* } $($tt:tt)*)
        ) => {
            let mut __thorn_first = true;
            for $pat in $($iter)* {
                if !__thorn_first {
                    $out.inner().write_str(($($join,)? ",",).0)?;
                }
                __thorn_first = false;

                __isql!([] $out; $($rest)*);
            }

            __isql!([] $out; $($tt)*);
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
        //         __isql!([] $out; $($then)*);
        //     } else {
        //         __isql!(@BRANCH $out;
        //             pred = ()
        //             rest = ($($rest)*)
        //         );
        //     }
        //     __isql!([] $out; $($tt)*);
        // };

        (@BRANCH $out:expr;
            pred = ( $($pred:tt)+ )
            rest = ( { $($then:tt)* } else { $($else:tt)* } $($tt:tt)* )
        ) => {
            if $($pred)* {
                __isql!([] $out; $($then)*);
            } else {
                __isql!([] $out; $($else)*);
            }
            __isql!([] $out; $($tt)*);
        };

        (@BRANCH $out:expr;
            pred = ( $($pred:tt)+ )
            rest = ( { $($then:tt)* } $($tt:tt)* )
        ) => {
            if $($pred)* {
                __isql!([] $out; $($then)*);
            }
            __isql!([] $out; $($tt)*);
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
            pred = ($($pred:tt)+)
            rest = ( {
                $($pat:pat $(if $pat_cond:expr)? => { $($pt:tt)* } $(,)?)*
            } $($tt:tt)* )
        ) => {
            match $($pred)* {$(
                $pat $(if $pat_cond)? => {
                    __isql!([] $out; $($pt)*);
                },
            )*}

            __isql!([] $out; $($tt)*);
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

        ([$($stack:literal)*] $out:expr; AS $table:ident.$column:ident $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)* "AS"]);
            std::write!($out.inner(), "\"{}\" ", <$table as $crate::table::Column>::name(&$table::$column))?;
            __isql!([] $out; $($tt)*);
        };
"#,
    )?;

    for keyword in src.split_whitespace() {
        writeln!(
            file,
            r#"([$($stack:literal)*] $out:expr; {keyword} $($tt:tt)*) => {{ __isql!([$($stack)* "{keyword}"] $out; $($tt)*); }};"#
        )?;
    }

    file.write_all(
        br##"
        ([$($stack:literal)*] $out:expr; $table:ident.$column:ident $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.write_column($table::$column)?;
            __isql!([] $out; $($tt)*);
        };

        ([$($stack:literal)*] $out:expr; $var:ident++; $($tt:tt)*) => {
            $var += 1;
            __isql!([$($stack)*] $out; $($tt)*);
        };
        ([$($stack:literal)*] $out:expr; $var:ident--; $($tt:tt)*) => {
            $var -= 1;
            __isql!([$($stack)*] $out; $($tt)*);
        };

        ([$($stack:literal)*] $out:expr; $table:ident $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.write_table::<$table>()?;
            __isql!([] $out; $($tt)*);
        };

        // parameters
        ([$($stack:literal)*] $out:expr; #{$param:expr $(=> $ty:expr)?} $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            {
                let param = $param;
                $out.param(param, ($($ty.into(),)? $crate::pg::Type::ANY,).0)?;
                std::write!($out, "${param}")?;
            }
            __isql!([] $out; $($tt)*);
        };

        // casts
        ([$($stack:literal)*] $out:expr; :: $param:ident $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            std::write!($out.inner(), "::{} ", stringify!($param))?;
            __isql!([] $out; $($tt)*);
        };

        ([$($stack:literal)*] $out:expr; () $($tt:tt)*) => {
            __isql!([$($stack)* "()"] $out; $($tt)*);
        };

        ([$($stack:literal)*] $out:expr; [] $($tt:tt)*) => {
            __isql!([$($stack)* "[]"] $out; $($tt)*);
        };

        // parenthesis and function calls
        ([$($stack:literal)*] $out:expr; .$func:ident ( $($it:tt)* ) $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.inner().write_str(stringify!($func))?;
            __isql!([] $out; ( $($it)* ));
            __isql!([] $out; $($tt)*);
        };

        ([$($stack:literal)*] $out:expr; ( $($it:tt)* ) $($tt:tt)*) => {
            __isql!([$($stack)* "("] $out; $($it)*);
            __isql!([")"] $out; $($tt)*);
        };

        // arbitrary runtime function calls
        ([$($stack:literal)*] $out:expr; .{$func:expr} ( $($it:tt)* ) $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            write!($out.inner(), "{}", $func)?;
            __isql!([] $out; ( $($it)* ) $($tt)*);
        };

        // square brackets/array
        ([$($stack:literal)*] $out:expr; [ $($it:tt)* ] $($tt:tt)*) => {
            __isql!([$($stack)* "["] $out; $($it)*);
            __isql!(["]"] $out; $($tt)*);
        };

        // arbitrary runtime expressions
        ([$($stack:literal)*] $out:expr; @$value:block $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            std::write!($out, "{}", $value)?;
            __isql!([] $out; $($tt)*);
        };

        // arbitrary runtime type casting
        ([$($stack:literal)*] $out:expr; ::$value:block $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            std::write!($out, "::{}", $crate::pg::Type::from($value))?;
            __isql!([] $out; $($tt)*);
        };

        ([$($stack:literal)*] $out:expr; !$block:block $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $block;
            __isql!([] $out; $($tt)*);
        };
    "##,
    )?;

    for token in TOKENS {
        writeln!(
            file,
            r#"([$($stack:literal)*] $out:expr; {token} $($tt:tt)*) => {{ __isql!([$($stack)* "{token}"] $out; $($tt)*); }};"#
        )?;
    }

    file.write_all(
        br##"
        // arbitrary runtime literals
        ([$($stack:literal)*] $out:expr; $value:block $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.write_literal($value)?;
            __isql!([] $out; $($tt)*);
        };

        ([$($stack:literal)*] $out:expr;) => {
            __isql!(@FLUSH $out; [$($stack)*]);
        };
}"##,
    )?;

    Ok(())
}

const TOKENS: &[&str] = &[
    "->>", "#>>", "/||", "@@", "@>", "<@", "^@", "/|", "&&", "||", "()", "[]", "!!", "->", "#>", "<<", ">>",
    "<>", "!=", ">=", "<=", ">", "<", "#", "~", "^", "|", "&", "%", "/", "*", "-", "+", "=", "!", ",", ";",
];
