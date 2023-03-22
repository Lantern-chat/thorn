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
        ([$($stack:expr)*] $out:expr; -- $($tt:tt)*) => { __isql!([$($stack)* "$$"] $out; $($tt)*); };
        ([$($stack:expr)*] $out:expr; 1 $($tt:tt)*) => { __isql!([$($stack)* "1"] $out; $($tt)*); };

        (@FLUSH $out:expr; [$($stack:expr)+]) => {
            $out.inner().write_str(concat!($($stack, " ",)*))?;
        };

        (@FLUSH $out:expr; []) => {};

        ([$($stack:expr)*] $out:expr; $lit:literal $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.write_literal($lit)?; __isql!([] $out; $($tt)*);
        };

        ([$($stack:expr)*] $out:expr; break; $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            break;
        };

        ([$($stack:expr)*] $out:expr; continue; $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            continue;
        };

        ([$($stack:expr)*] $out:expr; return; $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            return Ok(());
        };

        ([$($stack:expr)*] $out:expr; let $pat:pat_param = $expr:expr; $($tt:tt)*) => {
            let $pat = $expr;
            __isql!([$($stack)*] $out; $($tt)*);
        };

        ([$($stack:expr)*] $out:expr; const $name:ident: $ty:ty = $expr:expr; $($tt:tt)*) => {
            const $name: $ty = $expr;
            __isql!([$($stack)*] $out; $($tt)*);
        };

        ([$($stack:expr)*] $out:expr; for-join $({$join:literal})? $pat:pat in $($rest:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            __isql!(@JOIN $out;
                pat = ($pat)
                join = ($($join)?)
                iter = ()
                rest = ($($rest)*)
            );
        };

        ([$($stack:expr)*] $out:expr; for $pat:pat in $($rest:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            __isql!(@FOR $out;
                pat = ($pat)
                iter = ()
                rest = ($($rest)*)
            );
        };

        ([$($stack:expr)*] $out:expr; if $($rest:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            __isql!(@BRANCH $out;
                pred = ()
                rest = ($($rest)*)
            );
        };

        ([$($stack:expr)*] $out:expr; match $($rest:tt)*) => {
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

        ([$($stack:expr)*] $out:expr; AS $table:ident.$column:ident $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)* "AS"]);
            $out.write_column_name($table::$column)?;
            __isql!([] $out; $($tt)*);
        };
"#,
    )?;

    for keyword in src.split_whitespace() {
        writeln!(
            file,
            r#"([$($stack:expr)*] $out:expr; {keyword} $($tt:tt)*) => {{ __isql!([$($stack)* "{keyword}"] $out; $($tt)*); }};"#
        )?;
    }

    file.write_all(
        br##"
        ([$($stack:expr)*] $out:expr; $table:ident.$column:ident $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.write_column($table::$column)?;
            __isql!([] $out; $($tt)*);
        };

        ([$($stack:expr)*] $out:expr; $var:ident++; $($tt:tt)*) => {
            $var += 1;
            __isql!([$($stack)*] $out; $($tt)*);
        };
        ([$($stack:expr)*] $out:expr; $var:ident--; $($tt:tt)*) => {
            $var -= 1;
            __isql!([$($stack)*] $out; $($tt)*);
        };

        ([$($stack:expr)*] $out:expr; $table:ident $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.write_table::<$table>()?;
            __isql!([] $out; $($tt)*);
        };

        // parameters
        ([$($stack:expr)*] $out:expr; #{$param:expr $(=> $ty:expr)?} $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            let __thorn_param = $param;
            $out.param(__thorn_param, ($($ty.into(),)? $crate::pg::Type::ANY,).0)?;
            std::write!($out, "${__thorn_param}")?;
            __isql!([] $out; $($tt)*);
        };

        // casts
        ([$($stack:expr)*] $out:expr; :: $param:ident $($tt:tt)*) => {
            __isql!([$($stack)* "::" stringify!($param)] $out; $($tt)*);
        };

        ([$($stack:expr)*] $out:expr; () $($tt:tt)*) => {
            __isql!([$($stack)* "()"] $out; $($tt)*);
        };

        ([$($stack:expr)*] $out:expr; [] $($tt:tt)*) => {
            __isql!([$($stack)* "[]"] $out; $($tt)*);
        };

        // parenthesis and function calls
        ([$($stack:expr)*] $out:expr; .$func:ident ( $($it:tt)* ) $($tt:tt)*) => {
            __isql!([$($stack)* stringify!($func)] $out; ( $($it)* ) $($tt)*);
        };

        ([$($stack:expr)*] $out:expr; (|) $($tt:tt)*) => {
            __isql!([$($stack)* ")"] $out; $($tt)* );
        };

        ([$($stack:expr)*] $out:expr; ( $($it:tt)* ) $($tt:tt)*) => {
            __isql!([$($stack)* "("] $out; $($it)* (|) $($tt)* );
        };

        // arbitrary runtime function calls
        ([$($stack:expr)*] $out:expr; .{$func:expr} ( $($it:tt)* ) $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            write!($out.inner(), "{}", $func)?;
            __isql!([] $out; ( $($it)* ) $($tt)*);
        };

        // square brackets/array
        ([$($stack:expr)*] $out:expr; [|] $($tt:tt)*) => {
            __isql!([$($stack)* "]"] $out; $($tt)*);
        };

        ([$($stack:expr)*] $out:expr; [ $($it:tt)* ] $($tt:tt)*) => {
            __isql!([$($stack)* "["] $out; $($it)* [|] $($tt)*);
        };

        // arbitrary runtime expressions
        ([$($stack:expr)*] $out:expr; @$value:block $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            std::write!($out, "{}", $value)?;
            __isql!([] $out; $($tt)*);
        };

        // arbitrary runtime type casting
        ([$($stack:expr)*] $out:expr; ::$value:block $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            std::write!($out, "::{}", $crate::pg::Type::from($value))?;
            __isql!([] $out; $($tt)*);
        };

        ([$($stack:expr)*] $out:expr; !$block:block $($tt:tt)*) => {
            $block;
            __isql!([$($stack)*] $out; $($tt)*);
        };
    "##,
    )?;

    for token in TOKENS {
        writeln!(
            file,
            r#"([$($stack:expr)*] $out:expr; {token} $($tt:tt)*) => {{ __isql!([$($stack)* "{token}"] $out; $($tt)*); }};"#
        )?;
    }

    file.write_all(
        br##"
        // arbitrary runtime literals
        ([$($stack:expr)*] $out:expr; $value:block $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.write_literal($value)?;
            __isql!([] $out; $($tt)*);
        };

        ([$($stack:expr)*] $out:expr;) => {
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
