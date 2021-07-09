use clap::{Arg, App};

use script::{
    vm::VM,
    errors::Result,
    lexer::Lexer,
    parser::Parser,
    compiler::Compiler,
    opcodes::Op,
};

fn print_code(code: &[Op]) {
    for i in 0 .. code.len() {
        println!("{:4}\t{:?}", i, code[i]);
    }
}

fn try_compiler(source: &str) -> Result<()> {
    let mut parser = Parser::new(Lexer::new(source))?;
    let mut compiler = Compiler::new();
    while let Some(ast) = parser.next()? {
        compiler.feed(&ast)?;
    }
    let code = compiler.build()?;
    println!("COMPILED>");
    print_code(&code);

    let mut vm = VM::new();

    println!("RUN>");
    vm.run(&code)?;
    vm.collect();
    println!("{:?}", vm);
    Ok(())
}

fn main() {
    let matches = App::new("script")
        .version("0.1")
        .author("TiagoTM <tiago.martinez@gmail.com>")
        .about("Script compiler and VM")
        .arg(Arg::with_name("source")
            .index(1)
            .help("Name of input source file")
            .required(true))
        .get_matches();

    let source_name = matches.value_of("source").unwrap();
    let source = std::fs::read_to_string(source_name).unwrap();

    if let Err(err) = try_compiler(&source) {
        eprintln!("error: {}", err.pretty(&source));
    }
}
