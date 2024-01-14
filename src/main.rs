use std::fs::File;
use std::io::Write;
use std::{collections::HashMap, io::BufRead};
use std::{env, io::BufReader};

use itertools::Itertools;

const PREDEFINED_SYMBOLS: [(&str, u16); 23] = [
    ("SP", 0),
    ("LCL", 1),
    ("ARG", 2),
    ("THIS", 3),
    ("THAT", 4),
    ("R0", 0),
    ("R1", 1),
    ("R2", 2),
    ("R3", 3),
    ("R4", 4),
    ("R5", 5),
    ("R6", 6),
    ("R7", 7),
    ("R8", 8),
    ("R9", 9),
    ("R10", 10),
    ("R11", 11),
    ("R12", 12),
    ("R13", 13),
    ("R14", 14),
    ("R15", 15),
    ("SCREEN", 16384),
    ("KBD", 24576),
];

fn destinations(dest: &str) -> &str {
    match dest {
        "" => "000",
        "M" => "001",
        "D" => "010",
        "MD" => "011",
        "A" => "100",
        "AM" => "101",
        "AD" => "110",
        "AMD" => "111",
        _ => panic!("Invalid dest: {}", dest),
    }
}

fn computations(comp: &str) -> &str {
    match comp {
        "0" => "0101010",
        "1" => "0111111",
        "-1" => "0111010",
        "D" => "0001100",
        "A" => "0110000",
        "M" => "1110000",
        "!D" => "0001101",
        "!A" => "0110001",
        "!M" => "1110001",
        "-D" => "0001111",
        "-A" => "0110011",
        "-M" => "1110011",
        "D+1" => "0011111",
        "A+1" => "0110111",
        "M+1" => "1110111",
        "D-1" => "0001110",
        "A-1" => "0110010",
        "M-1" => "1110010",
        "D+A" => "0000010",
        "D+M" => "1000010",
        "D-A" => "0010011",
        "D-M" => "1010011",
        "A-D" => "0000111",
        "M-D" => "1000111",
        "D&A" => "0000000",
        "D&M" => "1000000",
        "D|A" => "0010101",
        "D|M" => "1010101",
        _ => panic!("Invalid comp: {}", comp),
    }
}

fn jumps(jump: &str) -> &str {
    match jump {
        "" => "000",
        "JGT" => "001",
        "JEQ" => "010",
        "JGE" => "011",
        "JLT" => "100",
        "JNE" => "101",
        "JLE" => "110",
        "JMP" => "111",
        _ => panic!("Invalid jump: {}", jump),
    }
}

struct SymbolTable<'data> {
    labels: HashMap<&'data str, u16>,
    variables: HashMap<&'data str, u16>,
    variable_address: u16,
}

impl<'data> SymbolTable<'data> {
    // by taking an `Iterator`, we guarantee to our caller that we
    // iterate at most once
    fn new<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'data String>,
    {
        let mut labels = HashMap::from(PREDEFINED_SYMBOLS);
        let mut program_length = 0; // where labels point to

        for line in iter.into_iter() {
            let line = line.trim();
            if line.starts_with('(') {
                // line is a label
                let label = line.trim_start_matches('(').trim_end_matches(')');
                labels.insert(label, program_length);
            } else {
                // label lines shouldn't contribute to program length
                program_length += 1
            }
        }

        Self {
            labels,
            variables: HashMap::new(),
            variable_address: 16,
        }
    }

    fn label(&mut self, key: &'data str) -> Option<u16> {
        self.labels.get(key).copied()
    }

    // this function will always alloc a new variable if one doesn't already exist
    fn variable<'slf>(&'slf mut self, key: &'data str) -> u16 {
        let ret = self.variables.entry(key).or_insert(self.variable_address);
        self.variable_address += 1;
        *ret
    }
}

fn assemble(input: impl BufRead, output: &mut impl Write) -> Result<(), std::io::Error> {
    // read file into memory
    let lines: Result<Vec<_>, _> = input
        .lines()
        // filter out comments and empty lines
        .filter_ok(|line| !line.trim().starts_with("//") && !line.is_empty())
        .collect();
    let lines = lines?;

    // first pass: collect labels into a symbol table
    let mut symbols = SymbolTable::new(&lines);

    // second pass: generate binary instructions
    for line in &lines {
        let line = line.trim();
        if line.starts_with('(') {
            continue;
        }

        if line.starts_with('@') {
            // A-instruction
            let value = line.trim_start_matches('@');
            let address = if let Ok(value) = value.parse::<u16>() {
                // plain memory address
                value
            } else if let Some(address) = symbols.label(value) {
                // existing label
                address
            } else {
                // variable (whether existing or new)
                symbols.variable(value)
            };
            writeln!(output, "{:016b}", address)?;
        } else {
            // split C-instruction into dest, comp, and jump
            let (dest, comp, jump) = {
                let (dest, comp) = match line.split('=').collect_vec()[..] {
                    [comp] => ("", comp),
                    [dest, comp] => (dest, comp),
                    _ => panic!("more than one equal sign in instruction"),
                };

                let (comp, jump) = match comp.split(';').collect_vec()[..] {
                    [comp] => (comp, ""),
                    [comp, jump] => (comp, jump),
                    _ => panic!("more than one ; in instruction"),
                };

                (dest, comp, jump)
            };
            writeln!(
                output,
                "111{}{}{}",
                computations(comp),
                destinations(dest),
                jumps(jump)
            )?;
        };
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Please provide a file path as a command-line argument");
        return;
    }

    let input_file_path = &args[1];
    let mut input_file = File::open(input_file_path).expect("Error opening file");

    let output_file_path = format!("{}.hack", input_file_path.trim_end_matches(".asm"));
    let mut output_file = File::create(output_file_path).expect("Error creating output file");

    assemble(BufReader::new(&mut input_file), &mut output_file)
        .expect("Error writing to output file");

    println!("Done!");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::BufReader};

    #[test]
    fn rect() {
        let mut result = Vec::new();
        let mut rect = File::open("resources/Rect.asm").unwrap();
        assemble(BufReader::new(&mut rect), &mut result).unwrap();

        let expected = std::fs::read("resources/Rect.hack").unwrap();
        assert_eq!(result, expected);
    }
}
