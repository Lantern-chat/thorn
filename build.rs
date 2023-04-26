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
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; -- $($tt:tt)*) =>
            { __isql!([$($stack)* "$$"] ($($exports)*) $nested $out; $($tt)*); };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; 0 $($tt:tt)*) =>
            { __isql!([$($stack)* "0"] ($($exports)*) $nested $out; $($tt)*); };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; 1 $($tt:tt)*) =>
            { __isql!([$($stack)* "1"] ($($exports)*) $nested $out; $($tt)*); };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; 2 $($tt:tt)*) =>
            { __isql!([$($stack)* "2"] ($($exports)*) $nested $out; $($tt)*); };

        (@ERROR ,) => { compile_error!("Trailing commas are not supported in SQL") };
        (@ERROR $tt:tt) => { $tt };
        (@ERROR) => {};

        (@FLUSH $out:expr; [$($stack:expr)+]) => {
            $out.write_str(concat!($($stack, " ",)*));
        };

        (@FLUSH $out:expr; []) => {};

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $lit:literal $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.write_literal($lit)?; __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; use $($tt:tt)*) => {
            __isql!(@USE $nested $out;
                path = ()
                rest = ($($tt)*)
                stack = ($($stack)*)
                exports = ($($exports)*)
            );
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; break; $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            break;
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; continue; $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            continue;
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $macro:ident!($($it:tt)*); $($tt:tt)*) => {
            $macro!($($it)*);
            __isql!([$($stack)*] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $macro:ident!{$($it:tt)*}; $($tt:tt)*) => {
            $macro!{$($it)*};
            __isql!([$($stack)*] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $macro:ident![$($it:tt)*]; $($tt:tt)*) => {
            $macro![$($it)*];
            __isql!([$($stack)*] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; let $pat:pat_param = $expr:expr; $($tt:tt)*) => {
            let $pat = $expr;
            __isql!([$($stack)*] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; const $name:ident: $ty:ty = $expr:expr; $($tt:tt)*) => {
            const $name: $ty = $expr;
            __isql!([$($stack)*] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; for-join $({$join:literal})? $pat:pat in $($rest:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            __isql!(@JOIN $nested $out;
                pat = ($pat)
                join = ($($join)?)
                iter = ()
                rest = ($($rest)*)
                exports = ($($exports)*)
            );
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; for $pat:pat in $($rest:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            __isql!(@FOR $nested $out;
                pat = ($pat)
                iter = ()
                rest = ($($rest)*)
                exports = ($($exports)*)
            );
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; if $($rest:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            __isql!(@BRANCH $nested $out;
                pred = ()
                rest = ($($rest)*)
                exports = ($($exports)*)
            );
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; match $($rest:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            __isql!(@MATCH $nested $out;
                pred = ()
                rest = ($($rest)*)
                exports = ($($exports)*)
            );
        };

        (@USE $nested:ident $out:expr;
            path = ($($path:tt)*)
            rest = ($rest:tt; $($tt:tt)*)
            stack = ($($stack:expr)*)
            exports = ($($exports:ident)*)
        ) => {
            use $($path)* $rest;
            __isql!([$($stack)*] ($($exports)*) $nested $out; $($tt)*);
        };

        (@USE $nested:ident $out:expr;
            path = ($($path:tt)*)
            rest = ($next:tt $($rest:tt)*)
            stack = ($($stack:expr)*)
            exports = ($($exports:ident)*)
        ) => {
            __isql!(@USE $nested $out;
                path = ($($path)* $next)
                rest = ($($rest)*)
                stack = ($($stack)*)
                exports = ($($exports)*)
            );
        };

        (@FOR $nested:ident $out:expr;
            pat = ($pat:pat)
            iter = ($($iter:tt)+)
            rest = ({ $($rest:tt)* } $($tt:tt)*)
            exports = ($($exports:ident)*)
        ) => {
            for $pat in $($iter)* { __isql!([] () t $out; $($rest)*); }
            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        (@FOR $nested:ident $out:expr;
            pat = ($pat:pat)
            iter = ($($iter:tt)*)
            rest = ($next:tt $($rest:tt)*)
            exports = ($($exports:ident)*)
        ) => {
            __isql!(@FOR $nested $out;
                pat = ($pat)
                iter = ($($iter)* $next)
                rest = ($($rest)*)
                exports = ($($exports)*)
            );
        };

        (@JOIN $nested:ident $out:expr;
            pat = ($pat:pat)
            join = ($($join:literal)?)
            iter = ($($iter:tt)+)
            rest = ({ $($rest:tt)* } $($tt:tt)*)
            exports = ($($exports:ident)*)
        ) => {
            let mut __thorn_first = true;
            for $pat in $($iter)* {
                if !__thorn_first {
                    $out.write_str(($($join,)? ",",).0);
                }
                __thorn_first = false;
                __isql!([] () t $out; $($rest)*);
            }

            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        (@JOIN $nested:ident $out:expr;
            pat = ($pat:pat)
            join = ($($join:literal)?)
            iter = ($($iter:tt)*)
            rest = ($next:tt $($rest:tt)*)
            exports = ($($exports:ident)*)
        ) => {
            __isql!(@JOIN $nested $out;
                pat = ($pat)
                join = ($($join)?)
                iter = ($($iter)* $next)
                rest = ($($rest)*)
                exports = ($($exports)*)
            );
        };

        (@BRANCH $nested:ident $out:expr;
            pred = ( $($pred:tt)+ )
            rest = ( { $($then:tt)* } else { $($else:tt)* } $($tt:tt)* )
            exports = ($($exports:ident)*)
        ) => {
            if $($pred)* {
                __isql!([] () t $out; $($then)*);
            } else {
                __isql!([] () t $out; $($else)*);
            }
            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        (@BRANCH $nested:ident $out:expr;
            pred = ( $($pred:tt)+ )
            rest = ( { $($then:tt)* } $($tt:tt)* )
            exports = ($($exports:ident)*)
        ) => {
            if $($pred)* {
                __isql!([] () t $out; $($then)*);
            }
            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        (@BRANCH $nested:ident $out:expr;
            pred = ($($pred:tt)*)
            rest = ($next:tt $($rest:tt)*)
            exports = ($($exports:ident)*)
        ) => {
            __isql!(@BRANCH $nested $out;
                pred = ($($pred)* $next)
                rest = ($($rest)*)
                exports = ($($exports)*)
            );
        };

        (@MATCH $nested:ident $out:expr;
            pred = ($($pred:tt)+)
            rest = ( {
                $($pat:pat $(if $pat_cond:expr)? => { $($pt:tt)* } $(,)?)*
            } $($tt:tt)* )
            exports = ($($exports:ident)*)
        ) => {
            match $($pred)* {$(
                $pat $(if $pat_cond)? => {
                    __isql!([] () t $out; $($pt)*);
                },
            )*}

            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        (@MATCH $nested:ident $out:expr;
            pred = ($($pred:tt)*)
            rest = ($next:tt $($rest:tt)*)
            exports = ($($exports:ident)*)
        ) => {
            __isql!(@MATCH $nested $out;
                pred = ($($pred)* $next)
                rest = ($($rest)*)
                exports = ($($exports)*)
            );
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; INSERT INTO $table:ident $(AS $alias:ident)? ( $($t:ident.$column:ident),* $(,)? ) $($tt:tt)*) => {
            compile_error!("INSERT columns must use only column names, not table.column");
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; SET ( $($table:ident.$column:ident),* $(,)? ) $($tt:tt)*) => {
            compile_error!("UPDATE SET columns must use only column names, not table.column");
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; INSERT INTO $table:ident $(AS $alias:ident)? ( $($column:ident),* , ) $($tt:tt)*) => {
            __isql!(@ERROR ,);
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; INSERT INTO $table:ident $(AS $alias:ident)? ( $($column:ident),* ) $($tt:tt)*) => {
            // $table () can be interpreted as a function call without context, so go ahead and write the correct syntax here
            __isql!(@FLUSH $out; [$($stack)* "INSERT INTO"]);
            $out.write_table::<$table>()?;
            $( type $alias = $table; )? // same as regular aliases
            __isql!([] ($($exports)*) $nested $out; ( $($table./$column),* ) $($tt)* );
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; UPDATE ONLY $table:ident $(AS $alias:ident)? SET ( $($column:ident),* , ) $($tt:tt)*) => {
            __isql!(@ERROR ,);
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; UPDATE $table:ident $(AS $alias:ident)? SET ( $($column:ident),* , ) $($tt:tt)*) => {
            __isql!(@ERROR ,);
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; UPDATE ONLY $table:ident $(AS $alias:ident)? SET ( $column:ident ) $($tt:tt)*) => {
            __isql!([$($stack)* "UPDATE ONLY"] ($($exports)*) $nested $out; $table $(AS $alias)? SET $table./$column $($tt)* );
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; UPDATE ONLY $table:ident $(AS $alias:ident)? SET ( $($column:ident),* ) $($tt:tt)*) => {
            __isql!([$($stack)* "UPDATE ONLY"] ($($exports)*) $nested $out; $table $(AS $alias)? SET ( $($table./$column),* ) $($tt)* );
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; UPDATE $table:ident $(AS $alias:ident)? SET ( $column:ident ) $($tt:tt)*) => {
            __isql!([$($stack)* "UPDATE"] ($($exports)*) $nested $out; $table $(AS $alias)? SET $table./$column $($tt)* );
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; UPDATE $table:ident $(AS $alias:ident)? SET ( $($column:ident),* ) $($tt:tt)*) => {
            __isql!([$($stack)* "UPDATE"] ($($exports)*) $nested $out; $table $(AS $alias)? SET ( $($table./$column),* ) $($tt)* );
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; UPDATE ONLY $table:ident $(AS $alias:ident)? SET $($tt:tt)*) => {
            compile_error!("You must use the multi-column assignment form of UPDATE: UPDATE Table SET (Col, Col) = (Expr, Expr)");
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; UPDATE $table:ident $(AS $alias:ident)? SET $($tt:tt)*) => {
            compile_error!("You must use the multi-column assignment form of UPDATE: UPDATE Table SET (Col, Col) = (Expr, Expr)");
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; DO UPDATE SET $($tt:tt)*) => {
            compile_error!("Use `DO UPDATE TableName SET` instead for conflict actions.");
        };

        // modified syntax for conflict actions, DO UPDATE Table SET (Col, Col) = (Value, Value)
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; DO UPDATE $table:ident SET ( $($column:ident),* , ) $($tt:tt)*) => {
            __isql!(@ERROR ,);
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; DO UPDATE $table:ident SET ( $column:ident ) $($tt:tt)*) => {
            __isql!([$($stack)* "DO UPDATE SET"] ($($exports)*) $nested $out; $table./$column $($tt)* );
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; DO UPDATE $table:ident SET ( $($column:ident),* ) $($tt:tt)*) => {
            __isql!([$($stack)* "DO UPDATE SET"] ($($exports)*) $nested $out; ( $($table./$column),* ) $($tt)* );
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; DO UPDATE $table:ident SET $($tt:tt)*) => {
            compile_error!("You must use the multi-column assignment form of UPDATE: DO UPDATE Table SET (Col, Col) = (Expr, Expr)");
        };

        // WITH Table (Col, Col) AS ...
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $table:ident ( $($column:ident),* , ) AS $($tt:tt)*) => {
            __isql!(@ERROR ,);
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $table:ident ( $($column:ident),* ) AS $($tt:tt)*) => {
            __isql!([$($stack)*] ($($exports)*) $nested $out; $table ( $($table./$column),* ) AS $($tt)* );
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $table:ident.$column:ident AS @_ $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            paste::paste! {
                $out.write_column($table::$column, stringify!([<$table:snake>]))?;

                __isql!(
                    ["AS" concat!("\"", stringify!([<$table:snake _ $column:snake>]), "\"") ]
                    ($($exports)* [<$table $column>]) $nested $out;
                    $($tt)*
                );
            };
        };

        // `SELECT whatever AS @ExportedName` is the only valid syntax that does not conflict with absolute-value (@)
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; AS @$column:ident $($tt:tt)*) => {
            __isql!([ $($stack)* "AS" $crate::paste::paste!(concat!("\"", stringify!([<$column:snake>]), "\"")) ] ($($exports)* $column) $nested $out; $($tt)*);
        };

        // fixes syntax ambiguity with rule below this one
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $word:ident AS $table:ident.$column:ident $($tt:tt)*) => {
            __isql!([$($stack)*] () $nested $out; $word AS);
            $out.write_column_name($table::$column)?;
            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $table:ident AS $alias:ident $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            type $alias = $table;
            $out.write_table::<$table>()?;
            __isql!(["AS" paste::paste!(concat!("\"", stringify!([<$alias:snake>]), "\""))] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; AS $table:ident.$column:ident $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)* "AS"]);
            $out.write_column_name($table::$column)?;
            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };
"#,
    )?;

    for keyword in src.split_whitespace() {
        writeln!(
            file,
            r#"([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; {keyword} $($tt:tt)*) => {{ __isql!([$($stack)* "{keyword}"] ($($exports)*) $nested $out; $($tt)*); }};"#
        )?;
    }

    file.write_all(
        br##"
        // explicitely only use column name
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $table:ident./$column:ident $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            paste::paste! { $out.write_column_name($table::$column)?; }
            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $table:ident.$column:ident $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            paste::paste! { $out.write_column($table::$column, stringify!([<$table:snake>]))?; }
            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $var:ident++; $($tt:tt)*) => {
            $var += 1;
            __isql!([$($stack)*] ($($exports)*) $nested $out; $($tt)*);
        };
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $var:ident--; $($tt:tt)*) => {
            $var -= 1;
            __isql!([$($stack)*] ($($exports)*) $nested $out; $($tt)*);
        };

        // parameters
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; #{$param:expr => $ty:expr} $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.param($param, $ty.into())?;
            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        // casts
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; :: $param:ident $($tt:tt)*) => {
            __isql!([$($stack)* "::" stringify!($param)] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; () $($tt:tt)*) => {
            __isql!([$($stack)* "()"] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; [] $($tt:tt)*) => {
            __isql!([$($stack)* "[]"] ($($exports)*) $nested $out; $($tt)*);
        };

        // parenthesis and function calls
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $func:ident ( $($it:tt)* ) $($tt:tt)*) => {
            __isql!([$($stack)* stringify!($func)] ($($exports)*) $nested $out; ( $($it)* ) $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; , (|) $($tt:tt)*) => {
            __isql!(@ERROR ,);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; , [|] $($tt:tt)*) => {
            __isql!(@ERROR ,);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; (|) $($tt:tt)*) => {
            __isql!([$($stack)* ")"] ($($exports)*) $nested $out; $($tt)* );
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; ( $($it:tt)* ) $($tt:tt)*) => {
            __isql!([$($stack)* "("] ($($exports)*) $nested $out; $($it)* (|) $($tt)* );
        };

        (@VERIFY $func:ident
            params = ($($param:tt)*)
            rest = ( [$($it:tt)*] $($tt:tt)*)
        ) => {
            __isql!(@VERIFY $func
                params = ($($params)*)
                rest = ($($tt)*)
            );
        };

        (@VERIFY $func:ident
            params = ($($params:tt)*)
            rest = ( ( $($it:tt)* ) $($tt:tt)*)
        ) => {
            __isql!(@VERIFY $func
                params = ($($params)*)
                rest = ($($tt)*)
            );
        };

        (@VERIFY $func:ident
            params = ($($params:tt)*)
            rest = (, $($tt:tt)*)
        ) => {
            __isql!(@VERIFY $func
                params = ($($params)* , ())
                rest = ($($tt)*)
            );
        };

        (@VERIFY $func:ident
            params = ($($params:tt)+)
            rest = ($next:tt $($tt:tt)*)
        ) => {
            __isql!(@VERIFY $func
                params = ($($params)*)
                rest = ($($tt)*)
            );
        };

        (@VERIFY $func:ident
            params = ()
            rest = ($next:tt $($tt:tt)*)
        ) => {
            __isql!(@VERIFY $func
                params = (())
                rest = ($($tt)*)
            );
        };

        (@VERIFY $func:ident
            params = ($($params:tt)*)
            rest = ()
        ) => {
            $func::$func( $($params)* );
        };

        // arbitrary runtime function calls
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; .$func:ident ( $($it:tt)* ) $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.write_func::<$func>();

            __isql!(@VERIFY $func
                params = ()
                rest = ($($it)*)
            );

            __isql!([] ($($exports)*) $nested $out; ( $($it)* ) $($tt)*);
        };

        // square brackets/array
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; [|] $($tt:tt)*) => {
            __isql!([$($stack)* "]"] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; [ $($it:tt)* ] $($tt:tt)*) => {
            __isql!([$($stack)* "["] ($($exports)*) $nested $out; $($it)* [|] $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $table:ident $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.write_table::<$table>()?;
            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        // arbitrary runtime expressions
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; @$value:block $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            std::write!($out, "{}", $value)?;
            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        // arbitrary runtime type casting
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; ::$value:block $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            std::write!($out, "::{}", $crate::pg::Type::from($value))?;
            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; !$block:block $($tt:tt)*) => {
            $block;
            __isql!([$($stack)*] ($($exports)*) $nested $out; $($tt)*);
        };
    "##,
    )?;

    for token in TOKENS {
        writeln!(
            file,
            r#"([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; {token} $($tt:tt)*) => {{ __isql!([$($stack)* "{token}"] ($($exports)*) $nested $out; $($tt)*); }};"#
        )?;
    }

    file.write_all(
        br##"
        // arbitrary runtime literals
        ([$($stack:expr)*] ($($exports:ident)*) $nested:ident $out:expr; $value:block $($tt:tt)*) => {
            __isql!(@FLUSH $out; [$($stack)*]);
            $out.write_literal($value)?;
            __isql!([] ($($exports)*) $nested $out; $($tt)*);
        };

        ([$($stack:expr)*] ($($exports:ident)+) t $out:expr;) => {
            compile_error!("Column exports cannot be declared within branching code");
        };

        ([$($stack:expr)*] () t $out:expr;) => {
            __isql!(@FLUSH $out; [$($stack)*]);
        };

        ([$($stack:expr)*] () f $out:expr;) => {
            __isql!(@FLUSH $out; [$($stack)*]);
        };

        ([$($stack:expr)*] ($first_export:ident $($exports:ident)*) f $out:expr;) => {
            __isql!([$($stack)*] () f $out;);

            #[allow(clippy::enum_variant_names)]
            #[repr(usize)]
            enum ColumnIndices {
                $first_export = 0,
                $($exports,)*
            }

            $crate::paste::paste! {
                impl Columns {
                    #[inline(always)]
                    pub fn [<$first_export:snake>]<'a, T>(&'a self) -> Result<T, $crate::pgt::Error>
                    where T: $crate::pg::FromSql<'a>
                    { self.try_get(ColumnIndices::$first_export as usize) }

                    $(
                        #[inline(always)]
                        pub fn [<$exports:snake>]<'a, T>(&'a self) -> Result<T, $crate::pgt::Error>
                        where T: $crate::pg::FromSql<'a>
                        { self.try_get(ColumnIndices::$exports as usize) }
                    )*
                }
            }
        };
}"##,
    )?;

    Ok(())
}

const TOKENS: &[&str] = &[
    "->>", "#>>", "/||", "@@", "@>", "<@", "^@", "/|", "&&", "||", "()", "[]", "!!", "->", "#>", "<<", ">>", "<>",
    "!=", ">=", "<=", ">", "<", "#", "~", "^", "|", "&", "%", "/", "*", "-", "+", "=", "!", ",", ";",
];
