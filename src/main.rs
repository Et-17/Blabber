use clap::Parser;

mod grammar;
mod parser;
mod generator;
mod cli;

fn create_generation_closure(grammar: grammar::Grammar, start: Option<String>) -> Box<dyn Fn() -> generator::GenResult> {
    match start {
        Some(start_symbol) => Box::new(move || generator::generate_with_override(&grammar, &start_symbol)),
        None => Box::new(move || generator::generate(&grammar))
    }
}

fn main() {
    let args = cli::Cli::parse();
    let grammar_res = parser::parse_file(&args.file);
    if let Err(errors) = grammar_res {
        for error in errors {
            eprintln!("{}", error);
        }
        std::process::exit(1);
    }
    let grammar = grammar_res.unwrap();

    let generate = create_generation_closure(grammar, args.start);

    for _ in 0..args.amount.unwrap_or(1) {
        let generated_res = generate();
        if let Err(error) = generated_res {
            eprintln!("{}", error);
            std::process::exit(1);
        }
        println!("{}", generated_res.unwrap());
    }
}
