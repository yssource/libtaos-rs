#![feature(proc_macro_span)]

use proc_macro2::*;
use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::*;

use std::path::PathBuf;

fn fn_name(item: TokenStream) -> Ident {
    let mut tokens = item.into_iter();
    let found = tokens
        .find(|tok| {
            if let TokenTree::Ident(word) = tok {
                if word == "fn" {
                    true
                } else {
                    false
                }
            } else {
                false
            }
        })
        .is_some();

    if !found {
        panic!("the macro attribute applies only to functions")
    }

    match tokens.next() {
        Some(TokenTree::Ident(word)) => word,
        _ => panic!("failed to find function name"),
    }
}

fn fn_description(item: TokenStream) -> String {
    item.into_iter()
        .filter_map(|token| match token {
            TokenTree::Group(group) => {
                let token = group.stream();
                let tokens: Vec<_> = token.into_iter().collect();
                if tokens.len() != 3 {
                    return None;
                }

                match &tokens[0..3] {
                    [TokenTree::Ident(_ident), TokenTree::Punct(_punct), TokenTree::Literal(literal)]
                         =>
                    {
                        let description = literal.to_string().trim_matches('"').trim().to_string();
                        if description.is_empty() {
                            None
                        } else {
                            Some(description)
                        }
                    }
                    _ => {
                        dbg!(tokens);
                        None
                    },
                }
            }
            _ => None,
        })
        .next()
        .unwrap_or_default()
}

fn source_file() -> PathBuf {
    let span = proc_macro::Span::call_site();
    let source = span.source_file();
    source.path()
}

fn rewrite_test_case(item: TokenStream) -> TokenStream {
    let source_file = source_file();
    let source_file = source_file.display();
    let fn_name = fn_name(item.clone());
    let mut tokens: Vec<_> = item.into_iter().collect();
    let last = tokens.len() - 1;
    let code = tokens.swap_remove(last);
    // let code = tokens.last_mut().unwrap();
    match code {
        TokenTree::Group(group) => {
            let delimiter = group.delimiter();
            let stream = group.stream();
            let case: TokenStream = format!(
                "let _case_ = test_catalog::CaseIdentity::new(\"{}\", \"{}\");
                let _now_ = std::time::Instant::now();",
                source_file, fn_name
            )
            .parse()
            .unwrap();
            let pre: TokenStream = format!("test_catalog::pre(&_case_);").parse().unwrap();
            let post: TokenStream = format!(
                "let _elapsed_ = _now_.elapsed(); test_catalog::post(&_case_, &_elapsed_);"
            )
            .parse()
            .unwrap();
            let new_stream = TokenStream::from_iter([case, pre, stream, post]);
            let group = Group::new(delimiter, new_stream);
            let tree = TokenTree::Group(group);
            tokens.push(tree);
        }
        _ => panic!("no code that seems not a valid test case"),
    }

    TokenStream::from_iter(tokens)
}

#[proc_macro_attribute]
pub fn test_catalogue(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    test_catalog::init();
    let span = proc_macro::Span::call_site();
    let file = span.source_file().path();
    let line_min = span.start();
    let line_max = span.end();

    let tokens = TokenStream::from(attr)
        .into_iter()
        .filter(|token| match token {
            TokenTree::Punct(p) if p.as_char() == ',' => false,
            _ => true,
        })
        .collect::<Vec<_>>();
    if tokens.len() % 3 != 0 {
        panic!("The right usage is: #[test_catalog(attr = \"value\")]");
    }

    let item = TokenStream::from(item);
    let name = fn_name(item.clone());
    let mut description = fn_description(item.clone());
    let mut since = String::new();
    let mut compatible_version = String::new();

    // use itertools::Itertools;
    for chunk in tokens.chunks_exact(3) {
        match &chunk[0..3] {
            [attr, _, value] => {
                let attr = attr.to_string();
                let value = value.to_string().trim_matches('"').trim().to_string();
                match attr.as_str() {
                    "since" => {
                        since = value;
                    }
                    "description" => {
                        description = value;
                    }
                    "compatible_version" => {
                        compatible_version = value;
                    }
                    _ => panic!("unsupported attribute for test_catalog"),
                }
            }
            _ => (),
        }
    }

    test_catalog::catalogue(
        &file.display().to_string(),
        &name.to_string(),
        line_min.line,
        line_max.line,
        &description,
        &since,
        &compatible_version,
    );

    let item = rewrite_test_case(item.clone());

    let ret: TokenStream = quote_spanned! {
        proc_macro2::Span::call_site() =>
        #item
    }
    .into();

    dbg!(format!("{}", ret));
    ret.into()
}
