use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
    let mut file = BufWriter::new(File::create(&path).unwrap());

    let mut keywords = phf_codegen::Set::new();

    for keyword in include_str!("./keywords.txt").split_whitespace() {
        keywords.entry(keyword);
    }

    writeln!(file, "static KEYWORDS: phf::Set<&'static str> = {};", keywords.build())?;

    let parse_sql_operator = build_operators()?;

    file.write_all(parse_sql_operator.as_bytes())?;

    Ok(())
}

// TODO: Revisit this to generate an actually tree, rather than just grouping by first character
fn build_operators() -> Result<String, std::fmt::Error> {
    use std::fmt::Write;

    let mut out = String::new();

    out.write_str(
        r#"
    #[allow(clippy::nonminimal_bool, clippy::collapsible_if)]
    fn parse_sql_operator(input: ParseStream) -> syn::Result<Option<&'static str>> {
        "#,
    )?;

    let mut operators = [
        "~", "||", "|", "^@", "^", "@@", "@", "@>", ">>", ">=", ">", "=", "<@", "<>", "<=", "<<", "<", ";", "/||",
        "/|", "/", "|/", "||/", "->>", "->", "-", ",", "+", "*", "&&", "&", "%", "#>>", "#>", "#", "!=", "!!",
        "!", "$$", "<<=", ">>=", "&<", "<&", "-|-", "@-@", "<->", "<<|", "|>>", "&<|", "|&>", "<^", ">^", "?#",
        "?-", "?|", "?-|", "?||", "~=", "~*",
    ];

    // sort and order by longest first
    operators.sort();
    operators.reverse();

    let mut groups: HashMap<char, String> = HashMap::new();

    for op in operators {
        let group_char = op.chars().next().unwrap();

        let group = groups.entry(group_char).or_default();

        // TODO: Make this generate prettier code?
        group.push_str("if true ");
        for (i, c) in op.chars().enumerate().skip(1) {
            let suffix = match i {
                1 => "2",
                2 => "3",
                _ => unreachable!(),
            };

            write!(group, "&& input.peek{suffix}(Token![{c}]) ")?;
        }

        group.push_str("{\n");

        for c in op.chars() {
            writeln!(group, "input.parse::<Token![{c}]>()?;")?;
        }

        writeln!(group, "return Ok(Some(\"{op}\"));\n}}")?;
    }

    let len = groups.len();
    for (i, (c, g)) in groups.into_iter().enumerate() {
        writeln!(out, "if input.peek(Token![{c}]) {{\n    {g}\n}}")?;
        if (i + 1) < len {
            out.write_str(" else ")?;
        }
    }

    out.write_str("Ok(None)\n}")?;

    Ok(out)
}
