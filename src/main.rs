extern crate oovm;

use crate::parse::FlatTree;

#[derive(Debug)]
pub enum Error {
    OovmError(oovm::Error),
    ParseError(usize, String),
    IOError(std::io::Error),
    CompileError(usize, String, String),
    NotImplemented(String),
}
type Result<T> = std::result::Result<T, Error>;

fn make_files(files: Vec<String>) -> Result<Vec<String>> {
    oovm::make(files, "radia.ignore/il/", "radia.ignore/bin/").map_err(Error::OovmError)
}

pub mod parse;
pub mod compile;

pub const PRELUDE: &str = include_str!( "../radia.ignore/stdlib/core/prelude.rad");

impl ToString for Error {
    fn to_string(&self) -> String {
        match self {
            Error::OovmError(_) => format!("OOVM Error"),
            Error::ParseError(line, msg) => format!("Parse error at line {line}: {msg}"),
            Error::IOError(error) => format!("IO error: {error}"),
            Error::CompileError(line, filename, msg) => format!("Compile error at line {line} of {filename}: {msg}"),
            Error::NotImplemented(feature) => format!("{feature} Not Implemented"),
        }
    }
}

fn main() -> Result<()> {
    let program = std::fs::read_to_string("main.rad").map_err(Error::IOError)?;
    let ast: Result<FlatTree<parse::AToken>> = parse::parse(program);
    if ast.is_err() {
        let err = ast.unwrap_err();
        println!("{}", err.to_string());
        return Err(err);
    }
    let ast = ast.unwrap();
    let files: Result<Vec<(String, String)>> = compile::compile_program(ast, String::from("main.rad"));
    if files.is_err() {
        let err = files.unwrap_err();
        println!("{}", err.to_string());
        return Err(err);
    }
    let files = files.unwrap();
    let mut paths: Vec<String> = Vec::new();
    for (path, file) in files {
        paths.push(path.clone());
        std::fs::write(String::from("radia.ignore/il/")+path.as_str(), file).map_err(Error::IOError)?;
    }
    let mut paths = make_files(paths)?;
    let dependencies = vec![
        String::from("radia.ignore/stdbin/Function.type"),
        String::from("radia.ignore/stdbin/stdio.String.0.mod"),
    ];
    paths.extend(dependencies);
    let exit_code = oovm::exec(paths, 0).map_err(Error::OovmError)?;
    println!("exit code {}", exit_code);
    Ok(())
}
