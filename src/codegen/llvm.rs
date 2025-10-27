use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use inkwell::builder::Builder;
use inkwell::context::Context as LlvmContext;
use inkwell::module::Module;
use inkwell::passes::PassManager;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum, FunctionType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};
use inkwell::AddressSpace;
use inkwell::OptimizationLevel;

use crate::ast::{BinaryOp, Expr, Function, Literal, Program, Statement};
use crate::runtime::ffi;
use crate::runtime::symbol_registry::{FfiSignature, FfiType, SymbolRegistry};

pub struct CodegenOptions {
    pub emit_ir: bool,
    pub opt_level: CodegenOptLevel,
    pub enable_lto: bool,
}

impl Default for CodegenOptions {
    fn default() -> Self {
        Self {
            emit_ir: false,
            opt_level: CodegenOptLevel::Default,
            enable_lto: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CodegenOptLevel {
    None,
    Default,
    Aggressive,
}

impl From<CodegenOptLevel> for OptimizationLevel {
    fn from(value: CodegenOptLevel) -> Self {
        match value {
            CodegenOptLevel::None => OptimizationLevel::None,
            CodegenOptLevel::Default => OptimizationLevel::Default,
            CodegenOptLevel::Aggressive => OptimizationLevel::Aggressive,
        }
    }
}

pub struct BuildArtifact {
    pub binary: PathBuf,
    pub ir: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OtterType {
    Unit,
    Bool,
    I32,
    I64,
    F64,
    Str,
}

impl From<FfiType> for OtterType {
    fn from(value: FfiType) -> Self {
        match value {
            FfiType::Unit => OtterType::Unit,
            FfiType::Bool => OtterType::Bool,
            FfiType::I32 => OtterType::I32,
            FfiType::I64 => OtterType::I64,
            FfiType::F64 => OtterType::F64,
            FfiType::Str => OtterType::Str,
        }
    }
}

struct EvaluatedValue<'ctx> {
    ty: OtterType,
    value: Option<BasicValueEnum<'ctx>>,
}

impl<'ctx> EvaluatedValue<'ctx> {
    fn with_value(value: BasicValueEnum<'ctx>, ty: OtterType) -> Self {
        Self {
            ty,
            value: Some(value),
        }
    }
}

struct Variable<'ctx> {
    ptr: PointerValue<'ctx>,
    ty: OtterType,
}

struct FunctionContext<'ctx> {
    variables: HashMap<String, Variable<'ctx>>,
}

impl<'ctx> FunctionContext<'ctx> {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    fn get(&self, name: &str) -> Option<&Variable<'ctx>> {
        self.variables.get(name)
    }

    fn insert(&mut self, name: String, variable: Variable<'ctx>) {
        self.variables.insert(name, variable);
    }
}

pub fn current_llvm_version() -> Option<String> {
    Some("15.0".to_string())
}

pub fn build_executable(
    program: &Program,
    output: &Path,
    options: &CodegenOptions,
) -> Result<BuildArtifact> {
    let context = LlvmContext::create();
    let module = context.create_module("otter");
    let builder = context.create_builder();
    let registry = ffi::bootstrap_stdlib();
    let mut compiler = Compiler::new(&context, module, builder, registry);

    compiler.lower_program(program)?;
    compiler
        .module
        .verify()
        .map_err(|e| anyhow!("LLVM module verification failed: {e}"))?;

    if options.emit_ir {
        // Ensure IR snapshot happens before LLVM potentially mutates the module during codegen.
        compiler.cached_ir = Some(compiler.module.print_to_string().to_string());
    }

    Target::initialize_native(&InitializationConfig::default())
        .map_err(|e| anyhow!("failed to initialise LLVM target: {e}"))?;

    let triple = TargetMachine::get_default_triple();
    compiler.module.set_triple(&triple);

    let target = Target::from_triple(&triple)
        .map_err(|e| anyhow!("failed to create target from triple: {e}"))?;

    let optimization: OptimizationLevel = options.opt_level.into();
    let target_machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            optimization,
            RelocMode::Default,
            CodeModel::Default,
        )
        .ok_or_else(|| anyhow!("failed to create target machine"))?;

    compiler
        .module
        .set_data_layout(&target_machine.get_target_data().get_data_layout());

    compiler.run_default_passes(options.opt_level);

    let object_path = output.with_extension("o");
    target_machine
        .write_to_file(&compiler.module, FileType::Object, &object_path)
        .map_err(|e| {
            anyhow!(
                "failed to emit object file at {}: {e}",
                object_path.display()
            )
        })?;

    let mut cc = Command::new("cc");
    cc.arg(&object_path).arg("-o").arg(output);

    if options.enable_lto {
        cc.arg("-flto");
    }

    let status = cc.status().context("failed to invoke system linker (cc)")?;

    if !status.success() {
        bail!("linker invocation failed with status {status}");
    }

    fs::remove_file(&object_path).ok();

    Ok(BuildArtifact {
        binary: output.to_path_buf(),
        ir: compiler.cached_ir.take(),
    })
}

struct Compiler<'ctx> {
    context: &'ctx LlvmContext,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    cached_ir: Option<String>,
    symbol_registry: &'static SymbolRegistry,
}

impl<'ctx> Compiler<'ctx> {
    fn new(
        context: &'ctx LlvmContext,
        module: Module<'ctx>,
        builder: Builder<'ctx>,
        symbol_registry: &'static SymbolRegistry,
    ) -> Self {
        Self {
            context,
            module,
            builder,
            cached_ir: None,
            symbol_registry,
        }
    }

    fn lower_program(&mut self, program: &Program) -> Result<()> {
        if program.functions.is_empty() {
            bail!("program contains no functions");
        }

        for function in &program.functions {
            self.lower_function(function)?;
        }

        if !program.functions.iter().any(|f| f.name == "main") {
            bail!("entry function `main` not found");
        }

        Ok(())
    }

    fn lower_function(&mut self, function: &Function) -> Result<FunctionValue<'ctx>> {
        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[], false);
        let llvm_fn = self.module.add_function(&function.name, fn_type, None);
        let entry = self.context.append_basic_block(llvm_fn, "entry");
        self.builder.position_at_end(entry);

        let mut ctx = FunctionContext::new();

        for statement in &function.body {
            self.lower_statement(statement, llvm_fn, &mut ctx)?;
        }

        if self
            .builder
            .get_insert_block()
            .and_then(|block| block.get_terminator())
            .is_none()
        {
            self.builder.build_return(Some(&i32_type.const_zero()));
        }

        Ok(llvm_fn)
    }

    fn lower_statement(
        &mut self,
        statement: &Statement,
        _function: FunctionValue<'ctx>,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<()> {
        match statement {
            Statement::Print(expr) => {
                let pointer = self.codegen_print_expr(expr, ctx)?;
                self.call_symbol("std.io.println", &[pointer.into()])?;
                Ok(())
            }
            Statement::Return(expr) => {
                if expr.is_some() {
                    bail!("return values are not supported yet");
                }
                let ret = self.context.i32_type().const_zero();
                self.builder.build_return(Some(&ret));
                Ok(())
            }
            Statement::Assignment { name, expr } => {
                let evaluated = self.eval_expr(expr, ctx)?;
                if evaluated.ty == OtterType::Unit {
                    bail!("cannot assign unit value to `{name}`");
                }

                let value = evaluated
                    .value
                    .clone()
                    .ok_or_else(|| anyhow!("expected value for assignment to `{name}`"))?;

                let ptr = if let Some(variable) = ctx.get(name) {
                    if variable.ty != evaluated.ty {
                        bail!(
                            "type mismatch assigning to `{name}`: existing {:?}, new {:?}",
                            variable.ty,
                            evaluated.ty
                        );
                    }
                    variable.ptr
                } else {
                    let ty = self.basic_type(evaluated.ty)?;
                    let alloca = self.builder.build_alloca(ty, name);
                    ctx.insert(
                        name.clone(),
                        Variable {
                            ptr: alloca,
                            ty: evaluated.ty,
                        },
                    );
                    alloca
                };

                self.builder.build_store(ptr, value);
                Ok(())
            }
        }
    }

    fn codegen_print_expr(
        &mut self,
        expr: &crate::ast::Expr,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<PointerValue<'ctx>> {
        let evaluated = self.eval_expr(expr, ctx)?;
        if evaluated.ty != OtterType::Str {
            bail!("print currently supports only string values");
        }

        let value = evaluated
            .value
            .ok_or_else(|| anyhow!("print expected a pointer value"))?;
        Ok(value.into_pointer_value())
    }

    fn eval_expr(
        &mut self,
        expr: &Expr,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        match expr {
            Expr::Literal(literal) => self.eval_literal(literal),
            Expr::Identifier(name) => {
                if let Some(variable) = ctx.get(name) {
                    let ty = self.basic_type(variable.ty)?;
                    let loaded = self.builder.build_load(ty, variable.ptr, name);
                    Ok(EvaluatedValue::with_value(loaded, variable.ty))
                } else {
                    bail!("unknown identifier `{name}`");
                }
            }
            Expr::Binary { left, op, right } => {
                let left_value = self.eval_expr(left, ctx)?;
                let right_value = self.eval_expr(right, ctx)?;

                if left_value.ty != OtterType::F64 || right_value.ty != OtterType::F64 {
                    bail!("binary expressions currently support only f64 operands");
                }

                let lhs = left_value
                    .value
                    .clone()
                    .ok_or_else(|| anyhow!("left operand missing value"))?
                    .into_float_value();
                let rhs = right_value
                    .value
                    .clone()
                    .ok_or_else(|| anyhow!("right operand missing value"))?
                    .into_float_value();

                let result = match op {
                    BinaryOp::Add => self.builder.build_float_add(lhs, rhs, "addtmp"),
                    BinaryOp::Sub => self.builder.build_float_sub(lhs, rhs, "subtmp"),
                    BinaryOp::Mul => self.builder.build_float_mul(lhs, rhs, "multmp"),
                    BinaryOp::Div => self.builder.build_float_div(lhs, rhs, "divtmp"),
                };

                Ok(EvaluatedValue::with_value(result.into(), OtterType::F64))
            }
            Expr::Call { callee, args } => self.eval_call(callee, args, ctx),
        }
    }

    fn eval_literal(&mut self, literal: &Literal) -> Result<EvaluatedValue<'ctx>> {
        match literal {
            Literal::String(value) => {
                let global = self.builder.build_global_string_ptr(value, "str");
                Ok(EvaluatedValue::with_value(
                    global.as_pointer_value().into(),
                    OtterType::Str,
                ))
            }
            Literal::Number(value) => {
                let float = self.context.f64_type().const_float(*value);
                Ok(EvaluatedValue::with_value(float.into(), OtterType::F64))
            }
        }
    }

    fn eval_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        match callee {
            Expr::Identifier(name) => {
                if let Some(symbol) = self.symbol_registry.resolve(name) {
                    if symbol.signature.params.len() != args.len() {
                        bail!(
                            "function `{name}` expected {} arguments but got {}",
                            symbol.signature.params.len(),
                            args.len()
                        );
                    }

                    let function = self.declare_symbol_function(name)?;
                    let mut lowered_args = Vec::with_capacity(args.len());

                    for (expr, expected) in args.iter().zip(symbol.signature.params.iter()) {
                        let value = self.eval_expr(expr, ctx)?;
                        let expected_ty: OtterType = expected.clone().into();
                        if value.ty != expected_ty {
                            bail!(
                                "argument type mismatch for `{name}`: expected {:?}, found {:?}",
                                expected_ty,
                                value.ty
                            );
                        }
                        lowered_args.push(self.value_to_metadata(&value)?);
                    }

                    let call_name = format!("call_{}", name.replace('.', "_"));
                    let call = self.builder.build_call(function, &lowered_args, &call_name);
                    let return_ty: OtterType = symbol.signature.result.into();
                    let value = match return_ty {
                        OtterType::Unit => None,
                        _ => Some(call.try_as_basic_value().left().ok_or_else(|| {
                            anyhow!("call to `{name}` did not produce a return value")
                        })?),
                    };
                    Ok(EvaluatedValue {
                        ty: return_ty,
                        value,
                    })
                } else if let Some(function) = self.module.get_function(name) {
                    if !args.is_empty() {
                        bail!("function `{name}` does not accept arguments yet");
                    }
                    let call = self
                        .builder
                        .build_call(function, &[], &format!("call_{name}"));
                    let value = call
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| anyhow!("call to `{name}` did not produce a value"))?;
                    Ok(EvaluatedValue::with_value(value, OtterType::I32))
                } else {
                    bail!("unknown function `{name}`");
                }
            }
            _ => bail!("only identifier calls are supported"),
        }
    }

    fn basic_type(&self, ty: OtterType) -> Result<BasicTypeEnum<'ctx>> {
        let ty = match ty {
            OtterType::Unit => bail!("unit type has no runtime representation"),
            OtterType::Bool => self.context.bool_type().into(),
            OtterType::I32 => self.context.i32_type().into(),
            OtterType::I64 => self.context.i64_type().into(),
            OtterType::F64 => self.context.f64_type().into(),
            OtterType::Str => self
                .context
                .i8_type()
                .ptr_type(AddressSpace::default())
                .into(),
        };
        Ok(ty)
    }

    fn value_to_metadata(
        &self,
        value: &EvaluatedValue<'ctx>,
    ) -> Result<BasicMetadataValueEnum<'ctx>> {
        let basic = value
            .value
            .clone()
            .ok_or_else(|| anyhow!("expected value for call argument"))?;
        Ok(basic.into())
    }

    fn call_symbol(&mut self, name: &str, args: &[BasicMetadataValueEnum<'ctx>]) -> Result<()> {
        let function = self.declare_symbol_function(name)?;
        let call_name = format!("call_{}", name.replace('.', "_"));
        self.builder.build_call(function, args, &call_name);
        Ok(())
    }

    fn declare_symbol_function(&mut self, name: &str) -> Result<FunctionValue<'ctx>> {
        let entry = self
            .symbol_registry
            .resolve(name)
            .ok_or_else(|| anyhow!("unresolved symbol `{name}`"))?;

        if let Some(function) = self.module.get_function(&entry.symbol) {
            return Ok(function);
        }

        let fn_type = self.ffi_signature_to_fn_type(&entry.signature)?;
        Ok(self.module.add_function(&entry.symbol, fn_type, None))
    }

    fn ffi_signature_to_fn_type(&self, signature: &FfiSignature) -> Result<FunctionType<'ctx>> {
        let params = self.ffi_param_types(&signature.params)?;
        let fn_type = match signature.result {
            FfiType::Unit => self.context.void_type().fn_type(&params, false),
            FfiType::Bool => self.context.bool_type().fn_type(&params, false),
            FfiType::I32 => self.context.i32_type().fn_type(&params, false),
            FfiType::I64 => self.context.i64_type().fn_type(&params, false),
            FfiType::F64 => self.context.f64_type().fn_type(&params, false),
            FfiType::Str => self
                .context
                .i8_type()
                .ptr_type(AddressSpace::default())
                .fn_type(&params, false),
        };
        Ok(fn_type)
    }

    fn ffi_param_types(&self, params: &[FfiType]) -> Result<Vec<BasicMetadataTypeEnum<'ctx>>> {
        params
            .iter()
            .map(|ty| self.ffi_type_to_basic(ty).map(Into::into))
            .collect()
    }

    fn ffi_type_to_basic(&self, ty: &FfiType) -> Result<BasicTypeEnum<'ctx>> {
        match ty {
            FfiType::Unit => bail!("unit type is not allowed in FFI parameter position"),
            FfiType::Bool => Ok(self.context.bool_type().into()),
            FfiType::I32 => Ok(self.context.i32_type().into()),
            FfiType::I64 => Ok(self.context.i64_type().into()),
            FfiType::F64 => Ok(self.context.f64_type().into()),
            FfiType::Str => Ok(self
                .context
                .i8_type()
                .ptr_type(AddressSpace::default())
                .into()),
        }
    }

    fn run_default_passes(&self, level: CodegenOptLevel) {
        if matches!(level, CodegenOptLevel::None) {
            return;
        }

        let pass_manager = PassManager::create(());
        pass_manager.add_instruction_combining_pass();
        pass_manager.add_reassociate_pass();
        pass_manager.add_gvn_pass();
        pass_manager.add_cfg_simplification_pass();
        pass_manager.add_instruction_simplify_pass();
        pass_manager.run_on(&self.module);
    }
}
