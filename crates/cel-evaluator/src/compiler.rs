use crate::compiled::CompiledCel;
use crate::errors::CompileError;
use crate::expr::rewrite_namespaced_calls;
use crate::security::SecurityLimits;
use cel::Program;

pub fn compile_expr(
    src: &str,
    limits: &SecurityLimits,
    path: String,
) -> Result<CompiledCel, CompileError> {
    limits.check_expr(src).map_err(CompileError::Mapping)?;
    let source = src.to_string();
    let rewritten = rewrite_namespaced_calls(src);
    let program = Program::compile(&rewritten).map_err(|err| CompileError::Cel {
        path,
        expression: source.clone(),
        message: err.to_string(),
    })?;
    Ok(CompiledCel { program, source })
}
