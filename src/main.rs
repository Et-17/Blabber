mod grammar;
mod parser;

fn main() {
    println!("This application is still in very early development and does not have a cli yet");
    println!("{:#?}", parser::parse_file(&"example_data/malformed.bnf").unwrap());
}
