#[macro_use]
extern crate clap;
extern crate cbor;
extern crate ast_importer;

use std::io::{Error, stdout, Cursor};
use std::io::prelude::*;
use std::fs::File;
use cbor::Decoder;
use ast_importer::clang_ast::process;
use ast_importer::c_ast::*;
use ast_importer::c_ast::Printer;
use ast_importer::clang_ast::AstContext;
use clap::{Arg, App};

fn main() {

    let matches = App::new("AST Importer")
        .version("0.1.0")
        .author(crate_authors!())

        // Testing related
        .arg(Arg::with_name("prefix-function-names")
            .long("prefix-function-names")
            .help("Adds a prefix to all function names. Generally only useful for testing")
            .takes_value(true))
        .arg(Arg::with_name("translate-entry")
            .long("translate-entry")
            .help("Creates an entry point that calls the C main function")
            .takes_value(false))

        // `AstContext` and `TypedAstContext` related
        .arg(Arg::with_name("dump-untyped-clang-ast")
            .long("ddump-untyped-clang-ast")
            .help("Prints out CBOR based Clang AST")
            .takes_value(false))
        .arg(Arg::with_name("dump-typed-clang-ast")
            .long("ddump-typed-clang-ast")
            .help("Prints out the parsed typed Clang AST")
            .takes_value(false))
        .arg(Arg::with_name("pretty-typed-clang-ast")
            .long("dpretty-typed-clang-ast")
            .help("Pretty-prints out the parsed typed Clang AST")
            .takes_value(false))
        .arg(Arg::with_name("translate-asm")
            .long("translate-asm")
            .help("Translate inline assembly without translating the assembly fragment")
            .takes_value(false))

        // CFG/Relooper related
        .arg(Arg::with_name("reloop-cfgs")
            .long("reloop-cfgs")
            .help("Translate function bodies using a CFG/Relooper approach")
            .takes_value(false))
        .arg(Arg::with_name("dump-function-cfgs")
            .long("ddump-function-cfgs")
            .help("Dumps into files DOT visualizations of the CFGs of every function")
            .takes_value(false))
        .arg(Arg::with_name("dump-structures")
            .long("ddump-structures")
            .help("Dumps out to STDERR the intermediate structures produced by relooper")
            .takes_value(false))
        .arg(Arg::with_name("debug-labels")
            .long("ddebug-labels")
            .help("Generate readable 'current_block' values in relooper")
            .takes_value(false))

        // Cross-check related
        .arg(Arg::with_name("cross-checks")
             .long("cross-checks")
             .help("Enable cross-checks")
             .takes_value(false))
        .arg(Arg::with_name("cross-check-config")
             .long("cross-check-config")
             .help("Add the given configuration files to the top-level #[cross_check(...)] attribute")
             .requires("cross-checks")
             .multiple(true)
             .takes_value(true))

        // End-user
        .arg(Arg::with_name("INPUT")
            .help("Sets the input CBOR file to use")
            .required(true)
            .index(1))
        .get_matches();

    let file = matches.value_of("INPUT").unwrap();
    let prefix_function_names = matches.value_of("prefix-function-names");
    let translate_entry = matches.is_present("translate-entry");
    let dump_untyped_context = matches.is_present("dump-untyped-clang-ast");
    let dump_typed_context = matches.is_present("dump-typed-clang-ast");
    let pretty_typed_context = matches.is_present("pretty-typed-clang-ast");
    let reloop_cfgs = matches.is_present("reloop-cfgs");
    let dump_function_cfgs = matches.is_present("dump-function-cfgs");
    let dump_structures = matches.is_present("dump-structures");
    let debug_labels = matches.is_present("debug-labels");
    let cross_checks = matches.is_present("cross-checks");
    let cross_check_configs = matches.values_of("cross-check-config")
        .map(|vals| vals.collect::<Vec<_>>())
        .unwrap_or_default();
    let translate_asm = matches.is_present("translate-asm");

    // Export the untyped AST to a CBOR file
    let untyped_context = match parse_untyped_ast(file) {
        Err(e) => panic!("{:#?}", e),
        Ok(cxt) => cxt,
    };

    if dump_untyped_context {
        println!("CBOR Clang AST");
        println!("{:#?}", untyped_context);
    }

    // Convert this into a typed AST
    let typed_context = {
        let mut conv = ConversionContext::new(&untyped_context);
        conv.convert(&untyped_context);
        conv.typed_context
    };

    if dump_typed_context {
        println!("Clang AST");
        println!("{:#?}", typed_context);
    }

    if pretty_typed_context {
        println!("Pretty-printed Clang AST");
        println!("{:#?}", Printer::new(stdout()).print(&typed_context));
    }


//    use syn::parse;
//    use quote::ToTokens;
//    use quote::Tokens;
//    if let parse::IResult::Done(_, t) = parse::ty("[u32; 10]") {
//        let mut tokens = Tokens::new();
//        t.to_tokens(&mut tokens);
//        println!("{}", tokens.as_str());
//    }

    // Perform the translation
    use ast_importer::translator::translate;

    let mut conv = ConversionContext::new(&untyped_context);
    conv.convert(&untyped_context);

    println!("{}", translate(
        conv.typed_context,
        reloop_cfgs,
        dump_function_cfgs,
        dump_structures,
        debug_labels,
        cross_checks,
        translate_asm,
        cross_check_configs,
        prefix_function_names,
        translate_entry,
    ));
}

fn parse_untyped_ast(filename: &str) -> Result<AstContext, Error> {
    let mut f = File::open(filename)?;
    let mut buffer = vec![];
    f.read_to_end(&mut buffer)?;

    let mut cursor: Decoder<Cursor<Vec<u8>>> = Decoder::from_bytes(buffer);
    let items = cursor.items();

    match process(items) {
        Ok(cxt) => Ok(cxt),
        Err(e) => panic!("{:#?}", e),
    }
}


