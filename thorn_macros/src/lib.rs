#![allow(clippy::single_char_add_str)]

extern crate proc_macro;

use heck::ToSnakeCase;
use proc_macro::TokenStream;
use proc_macro2::{Spacing, Span, TokenStream as TokenStream2, TokenTree};
use quote::ToTokens;

include!(concat!(env!("OUT_DIR"), "/codegen.rs"));

#[proc_macro]
pub fn __isql2(input: TokenStream) -> TokenStream {
    syn::parse_macro_input!(input with do_parse).into()
}

fn do_parse(input: ParseStream) -> syn::Result<TokenStream2> {
    let krate: Ident = input.parse()?;
    let writer = Ident::new("__thorn_query", Span::call_site());

    let mut state = State {
        krate: krate.clone(),
        writer: writer.clone(),
        ident: Default::default(),
        buffer: Default::default(),
        exports: Default::default(),
        cte: None,
        depth: 0,
        dynamic: false,
        params: Vec::new(),
    };

    let mut tokens = state.parse(input, &mut 0, false)?;

    let dynamic = state.dynamic;
    tokens.extend(quote::quote! {
        impl Columns {
            pub const IS_DYNAMIC: bool = #dynamic;
        }
    });

    //println!("{}", tokens.to_string());

    // final flush, after trim
    state.buffer.truncate(state.buffer.trim_end().len());
    if !state.buffer.is_empty() {
        let writer = &state.writer;
        let buffer = &state.buffer;
        tokens.extend(quote::quote! { #writer.write_str(#buffer); });
    }

    if let Some(first) = state.exports.first() {
        let mut buffer = state.ident;

        let accessors = state.exports.iter().map(|export| {
            let snake = buffer.ident(export).to_snake_case();

            let name = Ident::new(&snake, export.span());

            quote::quote! {
                #[inline(always)]
                pub fn #name<'a, T>(&'a self) -> Result<T, PgError>
                where T: FromSql<'a>
                { self.try_get(ColumnIndices::#export as usize) }
            }
        });

        let exports = state.exports.iter().skip(1);

        tokens.extend(quote::quote! {
            #[allow(clippy::enum_variant_names)]
            #[repr(usize)]
            enum ColumnIndices {
                #first = 0,
                #(#exports,)*
            }

            impl Columns {
                #(#accessors)*
            }
        });
    }

    let writer_ty = quote::quote! { #krate::macros::Query::<Columns> };

    if state.dynamic {
        tokens = quote::quote! {
            let mut #writer = #writer_ty::default();

            #tokens

            return Ok(#writer);
        };
    } else {
        let params = state.params.iter().map(|(v, _)| v);

        tokens = quote::quote! {
            static __QUERY: std::sync::OnceLock<Result<#krate::macros::StaticQuery<Columns>, #krate::macros::SqlFormatError>>
                = std::sync::OnceLock::new();

            return match __QUERY.get_or_init(|| {
                let mut #writer = #writer_ty::default();

                #tokens

                Ok(#writer.into())
            }) {
                Err(e) => Err(e.clone()),
                Ok(q) => Ok(#writer_ty::__from_cached(q, vec![#(#params),*])),
            };
        }
    }

    Ok(tokens)
}

mod lit;
//mod punct;

const TRAILING_COMMA: &str = "Trailing commas are not allowed in SQL";

use syn::{
    ext::IdentExt,
    parse::{Parse, ParseStream},
    punctuated::{Pair, Punctuated},
    token::{Brace, Bracket, Paren},
    Error, Expr, Ident, LitStr, Token,
};

mod kw {
    syn::custom_keyword!(INTO);
    syn::custom_keyword!(FROM);
    syn::custom_keyword!(AS);
    syn::custom_keyword!(SET);
    syn::custom_keyword!(UPDATE);
    syn::custom_keyword!(DO);
    syn::custom_keyword!(LATERAL);
    syn::custom_keyword!(NOT);
    syn::custom_keyword!(MATERIALIZED);
    syn::custom_keyword!(JOIN);
    syn::custom_keyword!(WHERE);

    syn::custom_keyword!(join);
}

#[derive(Default)]
struct IdentBuffer {
    buffer: String,
}

impl IdentBuffer {
    fn ident(&mut self, ident: &Ident) -> &str {
        use std::fmt::Write;

        self.buffer.clear();
        write!(self.buffer, "{ident}").unwrap();
        &self.buffer
    }

    fn take_ident(&mut self, ident: &Ident) -> String {
        self.ident(ident);
        std::mem::take(&mut self.buffer)
    }
}

struct State {
    krate: Ident,
    writer: Ident,
    ident: IdentBuffer,
    buffer: String,
    exports: indexmap::IndexSet<Ident>,
    cte: Option<Ident>,
    depth: usize,
    dynamic: bool,
    params: Vec<(Box<syn::Expr>, Box<syn::Type>)>,
}

impl State {
    fn ident(&mut self, ident: &Ident) -> &str {
        self.ident.ident(ident)
    }

    fn rewrite_spacing(&mut self) {
        let Some(start) = self.buffer.len().checked_sub(2) else {
            return;
        };

        let mut chars = self.buffer.chars().rev();

        let token = chars.next().unwrap();
        let maybe_ws = chars.next().unwrap();

        if maybe_ws.is_whitespace() && matches!(token, ',' | ')' | ']') {
            self.buffer.truncate(start);
            self.buffer.push(token);
        }
    }

    fn push(&mut self, tokens: impl ToTokens) {
        use std::fmt::Write;

        for t in tokens.into_token_stream() {
            let old_len = self.buffer.len();
            write!(self.buffer, "{}", t).unwrap();

            // single-byte token, check for punctuation rewrite
            if old_len + 1 == self.buffer.len() {
                self.rewrite_spacing();
            }

            if !matches!(t, TokenTree::Punct(ref p) if p.spacing() == Spacing::Joint) {
                self.buffer.push_str(" ")
            }
        }
    }

    fn push_str(&mut self, token: impl AsRef<str>) {
        let token = token.as_ref();
        self.buffer.push_str(token);
        if token.len() == 1 {
            self.rewrite_spacing();
        }
        if !matches!(token, "(" | "[" | "::") {
            self.buffer.push_str(" ");
        }
    }

    fn flush(&mut self, out: &mut TokenStream2) {
        if !self.buffer.is_empty() {
            let mut buffer = std::mem::take(&mut self.buffer);
            if !(buffer.ends_with([' ', '(', '[']) || buffer.ends_with("::")) {
                buffer.push_str(" ");
            }
            let writer = &self.writer;
            out.extend(quote::quote! { #writer.write_str(#buffer); });
        }
    }

    fn parse_nested(&mut self, input: ParseStream) -> syn::Result<TokenStream2> {
        self.depth += 1;
        let mut res = self.parse(input, &mut 0, true);
        if let Ok(ref mut out) = res {
            self.flush(out);
        }
        self.depth -= 1;
        res
    }

    fn assert_export_top(&self, span: Span) -> syn::Result<()> {
        if self.depth != 0 {
            return Err(Error::new(span, "Exports may only be defined in the top SQL scope"));
        }

        Ok(())
    }

    fn push_if_keyword(&mut self, ident: &Ident) -> bool {
        let ident = self.ident.take_ident(ident);

        if KEYWORDS.contains(&ident) {
            self.push_str(ident);
            return true;
        }

        false
    }

    fn parse_cte(&mut self, input: ParseStream, out: &mut TokenStream2, table: &Ident) -> syn::Result<()> {
        if input.peek(kw::NOT) {
            self.push(input.parse::<kw::NOT>()?);
        }

        if input.peek(kw::MATERIALIZED) {
            self.push(input.parse::<kw::MATERIALIZED>()?);
        }

        let inner;
        syn::parenthesized!(inner in input);

        self.cte = Some(table.clone());
        self.push_str("(");
        self.depth += 1;
        self.parse(&inner, &mut 0, false)?.to_tokens(out);
        self.depth -= 1;
        self.push_str(")");
        self.cte = None;

        Ok(())
    }

    fn format_column_list(
        &mut self,
        out: &mut TokenStream2,
        table: &Ident,
        mut columns: Punctuated<Ident, Token![,]>,
        naked_single: bool,
    ) -> syn::Result<()> {
        if let Some(trailing_comma) = columns.pop_punct() {
            return Err(Error::new(trailing_comma.span, TRAILING_COMMA));
        }

        let wrap = columns.len() > 1 || !naked_single;

        if wrap {
            self.push_str("(");
        }

        if !columns.is_empty() {
            self.flush(out);

            for pair in columns.pairs() {
                match pair {
                    Pair::Punctuated(col, comma) => {
                        self.write_column_name(out, table, col);
                        self.push(comma);
                    }
                    Pair::End(col) => self.write_column_name(out, table, col),
                }
            }
        }

        if wrap {
            self.push_str(")");
        }

        Ok(())
    }

    fn parse_set(&mut self, input: ParseStream, out: &mut TokenStream2, table: &Ident) -> syn::Result<()> {
        let set_token: kw::SET = input.parse()?;

        let inner;
        syn::parenthesized!(inner in input);

        let columns = inner.parse_terminated(Ident::parse, Token![,])?;

        self.push(set_token);
        self.format_column_list(out, table, columns, true)
    }

    fn parse_rename(&mut self, input: ParseStream, out: &mut TokenStream2, table: &Ident) -> syn::Result<Ident> {
        let as_token: kw::AS = input.parse()?;
        let alias: Ident = input.parse()?;

        let alias_name = self.ident(&alias).to_snake_case();

        self.push(as_token);
        self.push(LitStr::new(&alias_name, alias.span()));

        out.extend(quote::quote! { type #alias = #table; });

        Ok(alias)
    }

    fn parse_table_sequence(
        &mut self,
        input: ParseStream,
        out: &mut TokenStream2,
        table: &Ident,
    ) -> syn::Result<()> {
        // Ident() likely function call
        if input.peek(Paren) && !input.peek2(kw::AS) {
            self.push(table);
            return Ok(());
        }

        self.flush(out);
        self.write_table(out, table);

        match () {
            // Table SET ()
            _ if input.peek(kw::SET) && input.peek2(Paren) => {
                self.parse_set(input, out, table)?;
            }

            // Table AS [[NOT] MATERIALIZED] ()
            _ if input.peek(kw::AS)
                && (input.peek2(Paren)
                    || (input.peek2(kw::NOT) && input.peek3(kw::MATERIALIZED))
                    || input.peek2(kw::MATERIALIZED)) =>
            {
                let as_token: kw::AS = input.parse()?;
                self.push(as_token);
                self.parse_cte(input, out, table)?;
            }

            // Table AS Alias
            //
            // This adds on `AS "renamed"` and declared a type alias for other chunks
            // of code to reference
            //
            // Additionally parsed
            // Table AS Alias SET (Col)
            _ if input.peek(kw::AS) && input.peek2(Ident) => {
                let alias = self.parse_rename(input, out, table)?;

                if input.peek(kw::SET) && input.peek2(Paren) {
                    self.parse_set(input, out, &alias)?;
                }
            }

            // Table (Col1, Col2) AS
            _ if input.peek(Paren) && input.peek2(kw::AS) => {
                let inner;
                syn::parenthesized!(inner in input);

                let columns = inner.parse_terminated(Ident::parse, Token![,])?;

                let as_token: kw::AS = input.parse()?;

                self.format_column_list(out, table, columns, false)?;

                self.push(as_token);

                // we're going into a CTE, so track the current "table" during it
                if input.peek(Paren)
                    || (input.peek(kw::NOT) && input.peek2(kw::MATERIALIZED))
                    || input.peek(kw::MATERIALIZED)
                {
                    self.parse_cte(input, out, table)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn parse_ident_sequence(&mut self, input: ParseStream, out: &mut TokenStream2) -> syn::Result<()> {
        let ident: Ident = input.parse()?;

        match () {
            _ if self.push_if_keyword(&ident) => {}

            //_ if input.peek(Token![.]) && input.peek2(Token![*]) => {
            //    let _dot: Token![.] = input.parse()?;
            //    let _star: Token![*] = input.parse()?;
            //}

            // Table.Column
            _ if input.peek(Token![.]) && input.peek2(Ident) => {
                let dot: Token![.] = input.parse()?;
                let column: Ident = input.parse()?;

                // built-in pg_* namespace, ignore until better solution
                if ident.to_string().starts_with("pg_") {
                    self.push(ident);
                    self.push(dot);
                    self.push(column);

                    return Ok(());
                }

                let table_name = self.ident(&ident).to_snake_case();

                self.flush(out);
                let writer = &self.writer;
                out.extend(quote::quote! { #writer.write_column(#ident::#column, #table_name)?; });
                self.push_str(""); // empty space after column name

                // Table.Column AS @_
                //
                // Combines the table name and column name for an automatic export name, and also
                // sets an `AS "table_column"`
                if input.peek(kw::AS) && input.peek2(Token![@]) && input.peek3(Token![_]) {
                    let as_token: kw::AS = input.parse()?;
                    let _at_token: Token![@] = input.parse()?;
                    let underscore: Token![_] = input.parse()?;

                    let name = format!("{ident}{column}");

                    self.push(as_token);
                    self.push(LitStr::new(&name.to_snake_case(), underscore.span));

                    self.add_export(Ident::new(&name, underscore.span))?;
                }
            }

            // Table./Column syntax for unqualified column names
            _ if input.peek(Token![.]) && input.peek2(Token![/]) && input.peek3(Ident) => {
                let _dot: Token![.] = input.parse()?;
                let _slash: Token![/] = input.parse()?;
                let column: Ident = input.parse()?;

                self.flush(out);
                self.write_column_name(out, &ident, &column);
            }

            // Table
            _ => self.parse_table_sequence(input, out, &ident)?,
        }

        Ok(())
    }

    fn write_table(&mut self, out: &mut TokenStream2, table: &Ident) {
        self.flush(out);
        let writer = &self.writer;
        out.extend(quote::quote! { #writer.write_table::<#table>()?; })
    }

    fn write_column_name(&mut self, out: &mut TokenStream2, table: &Ident, column: &Ident) {
        self.flush(out);
        let writer = &self.writer;
        out.extend(quote::quote! { #writer.write_column_name(#table::#column)?; });
    }

    fn add_export(&mut self, name: Ident) -> syn::Result<()> {
        let span = name.span();
        self.assert_export_top(span)?;

        if let Some(prev) = self.exports.replace(name) {
            let mut err = Error::new(span, "Duplicate export");
            err.combine(Error::new(prev.span(), "Previously defined here"));

            return Err(err);
        }

        Ok(())
    }

    fn parse_inner(
        &mut self,
        input: ParseStream,
        comma_counter: &mut usize,
        allow_trailing: bool,
        out: &mut TokenStream2,
    ) -> syn::Result<()> {
        while !input.is_empty() {
            match () {
                // DO UPDATE SET
                _ if input.peek(kw::UPDATE) && input.peek2(kw::SET) && input.peek3(Paren) => {
                    return Err(input.error("Use `DO UPDATE TableName SET (col)` instead."));
                }

                // // SET Col =
                // _ if input.peek(kw::SET) && input.peek2(Ident) => {
                //     return Err(input.error("You must use the multi-column assignment form: UPDATE Table SET (Col, Col) = (Expr, Expr)\n\nSingle columns will be formatted correctly."));
                // }

                // INSERT INTO Table AS Alias (Col1, Col2)
                _ if input.peek(kw::INTO) && input.peek2(Ident) && (input.peek3(kw::AS) || input.peek3(Paren)) => {
                    let into_token: kw::INTO = input.parse()?;
                    let mut table: Ident = input.parse()?;

                    self.push(into_token);
                    self.write_table(out, &table);

                    if input.peek(kw::AS) {
                        if !input.peek2(Ident) {
                            return Err(input.error("Missing alias name after AS"));
                        }

                        table = self.parse_rename(input, out, &table)?;
                    }

                    let inner;
                    syn::parenthesized!(inner in input);

                    let columns = inner.parse_terminated(Ident::parse, Token![,])?;

                    self.format_column_list(out, &table, columns, false)?;
                }

                // DO UPDATE Table SET
                //
                // NOTE: This block is required because it's not a real syntax, and the Table name should
                // not be included
                _ if input.peek(kw::DO)
                    && input.peek2(kw::UPDATE)
                    && (input.peek3(Ident) && !input.peek3(kw::SET)) =>
                {
                    let do_token: kw::DO = input.parse()?;
                    let update_token: kw::UPDATE = input.parse()?;
                    let table: Ident = input.parse()?;
                    let set_token: kw::SET = input.parse()?;

                    self.push(do_token);
                    self.push(update_token);
                    self.push(set_token);

                    let inner;
                    syn::parenthesized!(inner in input);

                    let columns = inner.parse_terminated(Ident::parse, Token![,])?;

                    self.format_column_list(out, &table, columns, true)?;
                }

                // exports
                _ if input.peek(kw::AS) => {
                    let as_token: kw::AS = input.parse()?;

                    match () {
                        // AS @Name
                        _ if input.peek(Token![@]) && input.peek2(Ident) => {
                            let _at_token: Token![@] = input.parse()?;
                            let export: Ident = input.parse()?;

                            let name = self.ident(&export).to_snake_case();

                            self.push(as_token);
                            self.push(LitStr::new(&name, export.span()));

                            self.add_export(export)?;
                        }
                        // AS Table.Column
                        _ if input.peek(Ident) && input.peek2(Token![.]) && input.peek3(Ident) => {
                            let table: Ident = input.parse()?;
                            let _dot: Token![.] = input.parse()?;
                            let column: Ident = input.parse()?;

                            if let Some(ref cte) = self.cte {
                                if *cte != table {
                                    let mut err = Error::new(table.span(), "Conflicting CTE output column");
                                    err.combine(Error::new(cte.span(), "Defined here"));
                                    return Err(err);
                                }
                            }

                            self.push(as_token);
                            self.write_column_name(out, &table, &column);
                        }
                        _ => return Err(Error::new(as_token.span, "Unexpected AS")),
                    }
                }

                // handling this specially avoids ambiguity with other parts
                _ if (input.peek(kw::JOIN) || input.peek(kw::LATERAL))
                    && input.peek2(Paren)
                    && input.peek3(kw::AS) =>
                {
                    // JOIN or LATERAL
                    self.push(input.parse::<Ident>()?);

                    let inner;
                    syn::parenthesized!(inner in input);

                    let as_token: kw::AS = input.parse()?;
                    let alias: Ident = input.parse()?;

                    // while in the LATERAL join, check for this name instead
                    let old_cte = self.cte.replace(alias.clone());

                    // typical parenthesis
                    self.push_str("(");
                    self.depth += 1;
                    self.parse(&inner, &mut 0, false)?.to_tokens(out);
                    self.depth -= 1;
                    self.push_str(")");

                    self.cte = old_cte;

                    self.push(as_token);
                    self.write_table(out, &alias);
                }

                // .func(1, 2, 3)
                _ if input.peek(Token![.]) && input.peek2(Ident) && input.peek3(Paren) => {
                    let _: Token![.] = input.parse()?;
                    let ident: Ident = input.parse()?;

                    let args;
                    let parens = syn::parenthesized!(args in input);

                    let has_args = !args.is_empty();
                    let mut num_commas = 0;

                    self.flush(out);
                    let writer = &self.writer;
                    out.extend(quote::quote! { #writer.write_func::<#ident>(); });

                    // like () handling, but counts the commas in the subtree
                    self.push_str("(");
                    self.depth += 1;
                    self.parse(&args, &mut num_commas, false)?.to_tokens(out);
                    self.depth -= 1;
                    self.push_str(")");

                    // if we have arguments, commas + 1 is the number of arguments
                    num_commas += has_args as usize;

                    // <func>::func((), (), ()) but with correct span for parenthesis
                    out.extend(quote::quote!(<#ident>::#ident));
                    parens.surround(out, |tokens| {
                        let empty = quote::quote!(());
                        let empties = (0..num_commas).map(|_| &empty);
                        tokens.extend(quote::quote!(#(#empties,)*));
                    });
                    out.extend(quote::quote!(;));
                }

                _ if input.peek(Token![match]) => {
                    self.flush(out);
                    parse_match(input, self)?.to_tokens(out);
                }

                _ if input.peek(Token![if]) => {
                    self.flush(out);
                    parse_if(input, self)?.to_tokens(out);
                }

                _ if (input.peek(Token![for]) || input.peek(kw::join))
                    || (input.peek(syn::Lifetime)
                        && input.peek2(Token![:])
                        && (input.peek3(Token![for]) || input.peek3(kw::join))) =>
                {
                    self.flush(out);
                    parse_for(input, self)?.to_tokens(out, self);
                }

                _ if input.peek(Token![struct]) => {
                    let table = input.parse::<syn::ItemStruct>()?;
                    let krate = &self.krate;

                    out.extend(quote::quote! {
                        #krate::tables! { #table }
                    });
                }

                _ if is_macro(input) => {
                    input.parse::<syn::Stmt>()?.to_tokens(out);
                }

                _ if input.peek(Token![const]) && input.peek2(Brace) => {
                    let block = input.parse::<syn::PatConst>()?;

                    self.flush(out);
                    let writer = &self.writer;
                    out.extend(quote::quote! { #writer.write_literal(#block)?; });
                    self.push_str(""); // empty space after literal
                }

                // arbitrary const rust blocks
                _ if input.peek(Token![const]) && input.peek2(Token![$]) && input.peek3(Brace) => {
                    let const_ = input.parse::<Token![const]>()?;
                    let _bang: Token![$] = input.parse()?;

                    const_.to_tokens(out);
                    input.parse::<syn::Block>()?.to_tokens(out);
                }

                _ if input.peek(Ident) => {
                    self.parse_ident_sequence(input, out)?;
                }

                // SQL literals
                _ if input.peek(syn::Lit) => {
                    lit::push_lit(lit::parse_lit(input)?, self);
                }

                // parameters #{&value as Type::INT4}
                _ if input.peek(Token![#]) && input.peek2(Brace) => {
                    let _pound_token: Token![#] = input.parse()?;

                    let inner;
                    syn::braced!(inner in input);
                    let syn::ExprCast { expr, ty, .. } = inner.parse::<syn::ExprCast>()?;

                    self.flush(out);
                    let writer = &self.writer;
                    out.extend(
                        quote::quote! { #writer.param::<{Columns::IS_DYNAMIC}>((#expr) as _, #ty.into())?; },
                    );
                    self.params.push((expr, ty));
                    self.push_str(""); // space after param
                }

                _ if input.peek(Token![@]) && input.peek2(Brace) => {
                    let _at_token: Token![@] = input.parse()?;
                    let block: syn::Block = input.parse()?;

                    self.flush(out);
                    let writer = &self.writer;
                    out.extend(quote::quote! { write!(#writer, "{}", #block)?; });
                }

                // arbitrary Rust expressions ${x += 1;}
                _ if input.peek(Token![$]) && input.peek2(Brace) => {
                    let _bang: Token![$] = input.parse()?;
                    input.parse::<syn::Block>()?.to_tokens(out);
                }

                // SQL type casting
                _ if input.peek(Token![::]) => {
                    let _colon: Token![::] = input.parse()?;
                    self.push_str("::");

                    if input.peek(Ident) {
                        let ident: Ident = input.parse()?;

                        let mut ty = self.ident(&ident).to_uppercase();

                        // convert _TYPE to TYPE_ARRAY
                        if let Some(array_ty) = ty.strip_prefix('_') {
                            ty = format!("{array_ty}_ARRAY");
                        }

                        let ty_ident = Ident::new(&ty, ident.span());

                        self.flush(out);
                        let writer = &self.writer;
                        out.extend(quote::quote! { #writer.write_str(pg::Type::#ty_ident.name()); });

                        // cheat lack of space after this write_str by pushing an empty string
                        self.push_str("");
                    } else if input.peek(Brace) {
                        let block: syn::Block = input.parse()?;

                        self.flush(out);
                        let writer = &self.writer;
                        out.extend(quote::quote! { write!(#writer, "{} ", pg::Type::from(#block))?; });
                    }
                }

                // statements
                _ if is_stmt(input) => {
                    input.parse::<syn::Stmt>()?.to_tokens(out);
                }

                // deny other Rust keywords, if they aren't part of built-in syntax or arbitrary statements
                _ if is_rust_keyword(input) => {
                    return Err(input.error("Unexpected Rust keyword"));
                }

                // { ... }, runtime literals
                _ if input.peek(Brace) => {
                    let expr = input.parse::<syn::Block>()?;

                    self.flush(out);
                    let writer = &self.writer;
                    out.extend(quote::quote! { #writer.write_literal(#expr)?; });
                    self.push_str(""); // empty space after literal

                    self.dynamic = true;
                }

                // (...)
                _ if input.peek(Paren) => {
                    let inner;
                    syn::parenthesized!(inner in input);

                    self.push_str("(");
                    self.depth += 1;
                    self.parse(&inner, &mut 0, false)?.to_tokens(out);
                    self.depth -= 1;
                    self.push_str(")");
                }

                // [...]
                _ if input.peek(Bracket) => {
                    let inner;
                    syn::bracketed!(inner in input);

                    self.push_str("[");
                    self.depth += 1;
                    self.parse(&inner, &mut 0, false)?.to_tokens(out);
                    self.depth -= 1;
                    self.push_str("]");
                }

                // detect trailing commas
                _ if input.peek(Token![,]) => {
                    let comma: Token![,] = input.parse()?;

                    if (!allow_trailing && input.is_empty()) || (input.peek(kw::FROM) || input.peek(kw::WHERE)) {
                        return Err(Error::new(comma.span, TRAILING_COMMA));
                    }

                    self.push(comma);
                    *comma_counter += 1;
                }

                _ => {
                    // attempt to parse known SQL operators
                    if let Some(op) = parse_sql_operator(input)? {
                        self.push_str(op);
                    } else {
                        // passthrough as text
                        self.push(input.parse::<proc_macro2::TokenTree>()?);
                    }
                }
            }
        }

        Ok(())
    }

    fn parse(
        &mut self,
        input: ParseStream,
        comma_counter: &mut usize,
        allow_trailing: bool,
    ) -> syn::Result<TokenStream2> {
        let mut out = TokenStream2::new();

        let mut res = Ok(());

        syn::token::Brace::default().surround(&mut out, |out| {
            res = self.parse_inner(input, comma_counter, allow_trailing, out);
        });

        res.map(|_| out)
    }
}

fn is_rust_keyword(input: ParseStream) -> bool {
    input.peek(Ident::peek_any) && !input.peek(Ident)
}

fn is_macro(input: ParseStream) -> bool {
    if input.peek(Ident::peek_any) {
        if input.peek2(Token![!]) {
            return true;
        }

        let fork = input.fork();
        if fork.peek2(Token![::]) && syn::Path::parse_mod_style(&fork).is_ok() && fork.peek(Token![!]) {
            return true;
        }
    }

    false
}

// Supported subset of statements
fn is_stmt(input: ParseStream) -> bool {
    if input.peek(Token![let])
        || input.peek(Token![const])
        || input.peek(Token![use])
        || input.peek(Token![continue])
        || input.peek(Token![break])
        || input.peek(Token![type])
        || is_macro(input)
    {
        return true;
    }

    false
}

struct Match {
    match_token: Token![match],
    expr: Box<Expr>,
    brace_token: Brace,
    arms: Vec<Arm>,
}

struct Arm {
    pat: syn::Pat,
    guard: Option<(Token![if], Box<syn::Expr>)>,
    fat_arrow_token: Token![=>],
    brace_token: Brace,
    body: TokenStream2,
    comma: Option<Token![,]>,
}

enum Else {
    If(If),
    Block((Brace, TokenStream2)),
}

struct If {
    if_token: Token![if],
    cond: Box<Expr>,
    brace_token: Brace,
    then_branch: TokenStream2,
    else_branch: Option<(Token![else], Box<Else>)>,
}

struct For {
    label: Option<syn::Label>,
    for_token: Token![for],
    joiner: Option<syn::LitStr>,
    pat: Box<syn::Pat>,
    in_token: Token![in],
    expr: Box<Expr>,
    brace_token: Brace,
    body: TokenStream2,
}

fn parse_match(input: ParseStream, state: &mut State) -> syn::Result<Match> {
    let match_token = input.parse()?;
    let expr = Expr::parse_without_eager_brace(input)?;

    let content;
    let brace_token = syn::braced!(content in input);
    let mut arms = Vec::new();

    while !content.is_empty() {
        arms.push(parse_arm(&content, state)?);
    }

    if !arms.is_empty() {
        state.dynamic = true;
    }

    Ok(Match {
        match_token,
        expr: Box::new(expr),
        brace_token,
        arms,
    })
}

impl ToTokens for Match {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Match {
            match_token,
            expr,
            brace_token,
            arms,
        } = self;

        match_token.to_tokens(tokens);
        expr.to_tokens(tokens);

        brace_token.surround(tokens, |tokens| {
            for arm in arms {
                arm.to_tokens(tokens);
            }
        });
    }
}

fn parse_arm(input: ParseStream, state: &mut State) -> syn::Result<Arm> {
    let pat = syn::Pat::parse_multi_with_leading_vert(input)?;
    let guard = if input.peek(Token![if]) { Some((input.parse()?, input.parse()?)) } else { None };
    let fat_arrow_token = input.parse()?;
    let body;
    let brace_token = syn::braced!(body in input);
    let body = state.parse_nested(&body)?;
    let comma = input.parse()?;

    Ok(Arm {
        pat,
        guard,
        fat_arrow_token,
        brace_token,
        body,
        comma,
    })
}

impl ToTokens for Arm {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.pat.to_tokens(tokens);
        if let Some((ref if_token, ref cond)) = self.guard {
            if_token.to_tokens(tokens);
            cond.to_tokens(tokens);
        }
        self.fat_arrow_token.to_tokens(tokens);
        self.brace_token.surround(tokens, |tokens| {
            self.body.to_tokens(tokens);
        });
        self.comma.to_tokens(tokens);
    }
}

fn parse_if(input: ParseStream, state: &mut State) -> syn::Result<If> {
    let if_token = input.parse()?;
    let cond = Expr::parse_without_eager_brace(input)?;
    let then;
    let brace_token = syn::braced!(then in input);
    let then_branch = state.parse_nested(&then)?;

    let else_branch = if input.peek(Token![else]) {
        let else_token = input.parse()?;

        let else_branch = if input.peek(Token![if]) {
            Else::If(parse_if(input, state)?)
        } else {
            Else::Block({
                let inner;
                let brace_token = syn::braced!(inner in input);

                (brace_token, state.parse_nested(&inner)?)
            })
        };

        Some((else_token, Box::new(else_branch)))
    } else {
        None
    };

    state.dynamic = true;

    Ok(If {
        if_token,
        cond: Box::new(cond),
        brace_token,
        then_branch,
        else_branch,
    })
}

impl If {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let If {
            if_token,
            cond,
            brace_token,
            then_branch,
            else_branch,
        } = self;

        if_token.to_tokens(tokens);
        cond.to_tokens(tokens);

        brace_token.surround(tokens, |tokens| {
            then_branch.to_tokens(tokens);
        });

        if let Some((else_token, else_branch)) = else_branch {
            else_token.to_tokens(tokens);

            match **else_branch {
                Else::Block(ref block) => {
                    let (ref brace, ref block) = *block;
                    brace.surround(tokens, |tokens| block.to_tokens(tokens));
                }
                Else::If(ref else_if) => else_if.to_tokens(tokens),
            }
        }
    }
}

fn parse_for(input: ParseStream, state: &mut State) -> syn::Result<For> {
    let label = input.parse()?;

    let mut joiner = None;
    let for_token: Token![for] = if input.peek(kw::join) {
        let join_token = input.parse::<kw::join>()?;

        joiner = Some(if input.peek(Paren) {
            let joiner_input;
            syn::parenthesized!(joiner_input in input);

            joiner_input.parse()?
        } else {
            syn::LitStr::new(",", join_token.span)
        });

        Token![for](join_token.span)
    } else {
        input.parse()?
    };

    let pat = syn::Pat::parse_multi_with_leading_vert(input)?;

    let in_token = input.parse()?;
    let expr = Expr::parse_without_eager_brace(input)?;

    let inner;
    let brace_token = syn::braced!(inner in input);
    let body = state.parse_nested(&inner)?;

    state.dynamic = true;

    Ok(For {
        label,
        for_token,
        joiner,
        pat: Box::new(pat),
        in_token,
        expr: Box::new(expr),
        brace_token,
        body,
    })
}

impl For {
    fn to_tokens(&self, tokens: &mut TokenStream2, state: &State) {
        let For {
            label,
            for_token,
            joiner,
            pat,
            in_token,
            expr,
            brace_token,
            body,
        } = self;

        if joiner.is_some() {
            tokens.extend(quote::quote! { let mut __first = true; });
        }

        label.to_tokens(tokens);
        for_token.to_tokens(tokens);
        pat.to_tokens(tokens);
        in_token.to_tokens(tokens);
        expr.to_tokens(tokens);

        brace_token.surround(tokens, |tokens| {
            if let Some(ref joiner) = joiner {
                let writer = &state.writer;
                tokens.extend(quote::quote! {
                    if !__first { #writer.write_str(#joiner); }
                    __first = false;
                });
            }

            body.to_tokens(tokens);
        });
    }
}
