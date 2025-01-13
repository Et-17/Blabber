mod grammar;
mod parser;
mod generator;

fn main() {
    let grammar = parser::parse_file(&"example_data/postal_address_filled_in.bnf").unwrap();
    let generated = generator::generate(grammar).unwrap();
    println!("{:}", generated);
}
