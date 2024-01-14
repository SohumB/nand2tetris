use core::str::FromStr;
use std::error::Error;
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

trait Assemble {
    fn assemble<'slf>(
        &'slf self,
        table: &mut SymbolTable<'slf>,
        writer: &mut impl Write,
    ) -> Result<(), std::io::Error>;
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, parse_display::FromStr)]
enum Destination {
    Null,
    M,
    D,
    MD,
    A,
    AM,
    AD,
    AMD,
}

impl Assemble for Destination {
    fn assemble(
        &self,
        _table: &mut SymbolTable,
        writer: &mut impl Write,
    ) -> Result<(), std::io::Error> {
        write!(writer, "{:03b}", *self as u8)
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, parse_display::FromStr)]
enum Jump {
    Null,
    JGT,
    JEQ,
    JGE,
    JLT,
    JNE,
    JLE,
    JMP,
}

impl Assemble for Jump {
    fn assemble(
        &self,
        _table: &mut SymbolTable,
        writer: &mut impl Write,
    ) -> Result<(), std::io::Error> {
        write!(writer, "{:03b}", *self as u8)
    }
}

#[derive(Debug, Clone, Copy, parse_display::FromStr)]
enum AM {
    A,
    M,
}

impl Assemble for AM {
    fn assemble(
        &self,
        _table: &mut SymbolTable,
        writer: &mut impl Write,
    ) -> Result<(), std::io::Error> {
        write!(writer, "{:b}", *self as u8)
    }
}

#[derive(Debug, Clone, Copy)]
enum Computation {
    Zero,
    One,
    Neg1,
    D,
    X(AM),
    NegD,
    NegX(AM),
    DPlusOne,
    XPlusOne(AM),
    DMinusOne,
    XMinusOne(AM),
    DPlusX(AM),
    DMinusX(AM),
    XMinusD(AM),
    NotD,
    NotX(AM),
    DAndX(AM),
    DOrX(AM),
}

impl FromStr for Computation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Computation as C;
        match s {
            "0" => Ok(C::Zero),
            "1" => Ok(C::One),
            "-1" => Ok(C::Neg1),
            "D" => Ok(C::D),
            "A" => Ok(C::X(AM::A)),
            "M" => Ok(C::X(AM::M)),
            "!D" => Ok(C::NotD),
            "!A" => Ok(C::NotX(AM::A)),
            "!M" => Ok(C::NotX(AM::M)),
            "-D" => Ok(C::NegD),
            "-A" => Ok(C::NegX(AM::A)),
            "-M" => Ok(C::NegX(AM::M)),
            "D+1" => Ok(C::DPlusOne),
            "A+1" => Ok(C::XPlusOne(AM::A)),
            "M+1" => Ok(C::XPlusOne(AM::M)),
            "D-1" => Ok(C::DMinusOne),
            "A-1" => Ok(C::XMinusOne(AM::A)),
            "M-1" => Ok(C::XMinusOne(AM::M)),
            "D+A" => Ok(C::DPlusX(AM::A)),
            "D+M" => Ok(C::DPlusX(AM::M)),
            "D-A" => Ok(C::DMinusX(AM::A)),
            "D-M" => Ok(C::DMinusX(AM::M)),
            "A-D" => Ok(C::XMinusD(AM::A)),
            "M-D" => Ok(C::XMinusD(AM::M)),
            "D&A" => Ok(C::DAndX(AM::A)),
            "D&M" => Ok(C::DAndX(AM::M)),
            "D|A" => Ok(C::DOrX(AM::A)),
            "D|M" => Ok(C::DOrX(AM::M)),
            other => Err(format!("Invalid comp: {}", other)),
        }
    }
}

impl Assemble for Computation {
    fn assemble<'slf>(
        &'slf self,
        table: &mut SymbolTable<'slf>,
        writer: &mut impl Write,
    ) -> Result<(), std::io::Error> {
        use Computation as C;
        if let C::X(x)
        | C::NegX(x)
        | C::XPlusOne(x)
        | C::XMinusOne(x)
        | C::XMinusD(x)
        | C::DPlusX(x)
        | C::DMinusX(x)
        | C::NotX(x)
        | C::DAndX(x)
        | C::DOrX(x) = self
        {
            x.assemble(table, writer)?;
        } else {
            write!(writer, "0")?;
        };

        write!(
            writer,
            "{}",
            match self {
                Computation::Zero => "101010",
                Computation::One => "111111",
                Computation::Neg1 => "111010",
                Computation::D => "001100",
                Computation::X(_) => "110000",
                Computation::NegD => "001111",
                Computation::NegX(_) => "110011",
                Computation::DPlusOne => "011111",
                Computation::XPlusOne(_) => "110111",
                Computation::DMinusOne => "001110",
                Computation::XMinusOne(_) => "110010",
                Computation::DPlusX(_) => "000010",
                Computation::DMinusX(_) => "010011",
                Computation::XMinusD(_) => "000111",
                Computation::NotD => "001101",
                Computation::NotX(_) => "110001",
                Computation::DAndX(_) => "000000",
                Computation::DOrX(_) => "010101",
            }
        )
    }
}

#[derive(Debug, Clone)]
enum HackLine {
    Label(String),
    AImmediate(u16),
    ALocation(String),
    C(Computation, Destination, Jump),
}

impl FromStr for HackLine {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.split("//").next().unwrap_or(s).trim();
        if s.starts_with('(') {
            // line is a label
            let label = s.trim_start_matches('(').trim_end_matches(')');
            Ok(Self::Label(label.to_owned()))
        } else if s.starts_with('@') {
            // A-instruction
            let value = s.trim_start_matches('@');
            Ok(if let Ok(imm) = value.parse::<u16>() {
                // plain memory address
                Self::AImmediate(imm)
            } else {
                // location
                Self::ALocation(value.to_owned())
            })
        } else {
            // split C-instruction into dest, comp, and jump
            let (dest, comp, jump) = {
                let (dest, comp) = match s.split('=').collect_vec()[..] {
                    [comp] => (Destination::Null, comp),
                    [dest, comp] => (dest.parse()?, comp),
                    _ => Err("more than one equal sign in instruction")?,
                };

                let (comp, jump) = match comp.split(';').collect_vec()[..] {
                    [comp] => (comp, Jump::Null),
                    [comp, jump] => (comp, jump.parse()?),
                    _ => Err("more than one ; in instruction")?,
                };

                (dest, comp.parse()?, jump)
            };
            Ok(Self::C(comp, dest, jump))
        }
    }
}

impl Assemble for HackLine {
    fn assemble<'slf>(
        &'slf self,
        table: &mut SymbolTable<'slf>,
        writer: &mut impl Write,
    ) -> Result<(), std::io::Error> {
        match self {
            HackLine::Label(_) => {}
            HackLine::AImmediate(imm) => writeln!(writer, "{:016b}", imm)?,
            HackLine::ALocation(name) => {
                let address = if let Some(address) = table.label(name) {
                    // existing label
                    address
                } else {
                    // variable (allocating a new one if it doesn't already exist)
                    table.variable(name)
                };
                writeln!(writer, "{:016b}", address)?
            }
            HackLine::C(c, d, j) => {
                write!(writer, "111")?;
                c.assemble(table, writer)?;
                d.assemble(table, writer)?;
                j.assemble(table, writer)?;
                writeln!(writer)?;
            }
        }
        Ok(())
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
        I: IntoIterator<Item = &'data HackLine>,
    {
        let mut labels = HashMap::from(PREDEFINED_SYMBOLS);
        let mut program_length = 0; // where labels point to

        for line in iter.into_iter() {
            if let HackLine::Label(label) = line {
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

fn assemble(input: impl BufRead, output: &mut impl Write) -> Result<(), Box<dyn Error>> {
    // read file into memory
    let lines: Result<Vec<_>, _> = input
        .lines()
        // filter out comments and empty lines
        .filter_ok(|line| !line.trim().starts_with("//") && !line.is_empty())
        .map_ok(|line| line.parse::<HackLine>())
        .map(|res| res?)
        .collect();
    let lines = lines?;

    // first pass: collect labels into a symbol table
    let mut symbols = SymbolTable::new(&lines);

    // second pass: generate binary instructions
    for line in &lines {
        line.assemble(&mut symbols, output)?;
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
