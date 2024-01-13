use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{Read, Write};

fn predefined_symbols() -> HashMap<&'static str, u16> {
    let mut symbols: HashMap<&str, u16> = HashMap::new();
    symbols.insert("SP", 0);
    symbols.insert("LCL", 1);
    symbols.insert("ARG", 2);
    symbols.insert("THIS", 3);
    symbols.insert("THAT", 4);
    symbols.insert("R0", 0);
    symbols.insert("R1", 1);
    symbols.insert("R2", 2);
    symbols.insert("R3", 3);
    symbols.insert("R4", 4);
    symbols.insert("R5", 5);
    symbols.insert("R6", 6);
    symbols.insert("R7", 7);
    symbols.insert("R8", 8);
    symbols.insert("R9", 9);
    symbols.insert("R10", 10);
    symbols.insert("R11", 11);
    symbols.insert("R12", 12);
    symbols.insert("R13", 13);
    symbols.insert("R14", 14);
    symbols.insert("R15", 15);
    symbols.insert("SCREEN", 16384);
    symbols.insert("KBD", 24576);
    symbols
}

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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Please provide a file path as a command-line argument");
        return;
    }
    let file_path = &args[1];

    let mut file = File::open(file_path).expect("Error opening file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Error reading file");
    let contents = contents;

    let output_file_path = format!("{}.hack", file_path.trim_end_matches(".asm"));
    let mut dest_file = File::create(output_file_path).expect("Error creating output file");

    let mut symbol_table = predefined_symbols();
    let mut next_address: u16 = 16; // for user-defined symbols

    let mut program_length = 0; // where labels point to

    // first pass: add labels to symbol table
    for line in contents.lines() {
        // skip comments and empty lines
        if line.starts_with("//") || line.is_empty() {
            continue;
        }

        if line.starts_with("(") {
            // line is a label
            let label = line.trim_start_matches("(").trim_end_matches(")");
            symbol_table.insert(label, program_length);
        } else {
            // label lines shouldn't contribute to program length
            program_length += 1
        }
    }

    // second pass: generate binary instructions
    for line in contents.lines() {
        let line = line.trim();
        if line.starts_with("(") || line.starts_with("//") || line.is_empty() {
            continue;
        }

        let result = if line.starts_with("@") {
            // A-instruction
            let value = line.trim_start_matches("@");
            if let Ok(value) = value.parse::<u16>() {
                // plain memory address
                format!("{:016b}", value)
            } else if let Some(value) = symbol_table.get(value) {
                // existing symbol
                format!("{:016b}", value)
            } else {
                // new symbol
                let address = next_address;
                next_address += 1;
                symbol_table.insert(value, address);
                format!("{:016b}", address)
            }
        } else {
            // split C-instruction into dest, comp, and jump
            let mut dest = "";
            let mut comp = "";
            let mut jump = "";
            if line.contains("=") {
                // instruction has a destination
                let parts: Vec<&str> = line.split("=").collect();
                dest = parts[0];
                comp = parts[1];
            } else if line.contains(";") {
                // instruction has a jump
                let parts: Vec<&str> = line.split(";").collect();
                comp = parts[0];
                jump = parts[1];
            }
            dest = destinations(dest);
            comp = computations(comp);
            jump = jumps(jump);
            format!("111{}{}{}", comp, dest, jump)
        };

        dest_file
            .write_all(format!("{}\n", result).as_bytes())
            .expect("Error writing to output file");
    }

    println!("Done!");
}
