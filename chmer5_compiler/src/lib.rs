mod ast;
mod bytecode;
mod errors;
mod lexer;
mod parser;

pub use ast::*;
pub use bytecode::*;
pub use errors::*;
pub use lexer::*;
pub use parser::*;

pub fn compile_chmer(source_name: &str, src: &str) -> Result<Chunk, ChmerError> {
    let tokens = Lexer::new(source_name, src).lex()?;
    let program = Parser::new(source_name, src, tokens).parse_program()?;
    Compiler::new(source_name).compile_program(&program)
}

pub fn compile_ctl(source_name: &str, src: &str) -> Result<ModuleArtifact, ChmerError> {
    let tokens = Lexer::new(source_name, src).lex()?;
    let module = Parser::new(source_name, src, tokens).parse_ctl_module()?;
    Compiler::new(source_name).compile_ctl_module(&module)
}
