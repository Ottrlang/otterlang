use anyhow::{Result, bail};
use inkwell::values::{BasicValueEnum, FunctionValue};

use crate::codegen::llvm::compiler::Compiler;
use crate::codegen::llvm::compiler::types::{EvaluatedValue, FunctionContext, OtterType, Variable};
use crate::typecheck::TypeInfo;
use ast::nodes::{Block, Expr, Statement};

struct IteratorRuntime<'ctx> {
    create_fn: FunctionValue<'ctx>,
    has_next_fn: FunctionValue<'ctx>,
    next_fn: FunctionValue<'ctx>,
    free_fn: FunctionValue<'ctx>,
    element_type: OtterType,
}

impl<'ctx> Compiler<'ctx> {
    pub(crate) fn lower_block(
        &mut self,
        block: &Block,
        function: FunctionValue<'ctx>,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<()> {
        for stmt in &block.statements {
            self.lower_statement(stmt.as_ref(), function, ctx)?;
        }
        Ok(())
    }

    pub(crate) fn lower_statement(
        &mut self,
        stmt: &Statement,
        function: FunctionValue<'ctx>,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<()> {
        match stmt {
            Statement::Expr(expr) => {
                self.eval_expr(expr.as_ref(), ctx)?;
                Ok(())
            }
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    let val = self.eval_expr(expr.as_ref(), ctx)?;
                    if let Some(v) = val.value {
                        self.builder.build_return(Some(&v))?;
                    } else {
                        self.builder.build_return(None)?;
                    }
                } else {
                    self.builder.build_return(None)?;
                }
                Ok(())
            }
            Statement::Let {
                name,
                ty: _,
                expr,
                public: _,
            } => {
                let val = self.eval_expr(expr.as_ref(), ctx)?;
                let EvaluatedValue {
                    ty: val_ty,
                    value: val_value,
                } = val;

                // Skip allocation for Unit types
                if let Some(_basic_ty) = self.basic_type(val_ty.clone())? {
                    // Use create_entry_block_alloca to ensure alloca is in the entry block
                    // This prevents stack overflow in loops and ensures dominance
                    let function = self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_parent()
                        .unwrap();
                    let alloca =
                        self.create_entry_block_alloca(function, name.as_ref(), val_ty.clone())?;
                    if let Some(v) = val_value {
                        self.builder.build_store(alloca, v)?;
                    }
                    ctx.insert(
                        name.as_ref().to_string(),
                        Variable {
                            ptr: alloca,
                            ty: val_ty,
                        },
                    );
                }
                // For Unit types, we don't create a variable
                Ok(())
            }
            Statement::Assignment { name, expr } => {
                let val = self.eval_expr(expr.as_ref(), ctx)?;
                let EvaluatedValue {
                    ty: val_ty,
                    value: val_value,
                } = val;
                if let Some(var) = ctx.get(name.as_ref()) {
                    if let Some(v) = val_value {
                        // Type checking and coercion
                        let coerced_val = self.coerce_type(v, val_ty.clone(), var.ty.clone())?;
                        self.builder.build_store(var.ptr, coerced_val)?;
                    } else if val_ty != OtterType::Unit {
                        bail!(
                            "Cannot assign non-unit expression with no value to variable {}",
                            name.as_ref()
                        );
                    }
                    // Unit type assignments are no-ops
                } else {
                    bail!("Variable {} not declared", name.as_ref());
                }
                Ok(())
            }
            Statement::If {
                cond,
                then_block,
                elif_blocks,
                else_block,
            } => self.lower_if_statement(
                function,
                ctx,
                cond.as_ref(),
                then_block.as_ref(),
                elif_blocks,
                else_block.as_ref().map(|b| b.as_ref()),
            ),
            Statement::While { cond, body } => {
                self.lower_while_loop(function, ctx, cond.as_ref(), body.as_ref())
            }
            Statement::Break => {
                if let Some(loop_ctx) = ctx.current_loop() {
                    self.builder.build_unconditional_branch(loop_ctx.exit_bb)?;
                } else {
                    bail!("break statement outside of loop");
                }
                Ok(())
            }
            Statement::Continue => {
                if let Some(loop_ctx) = ctx.current_loop() {
                    self.builder.build_unconditional_branch(loop_ctx.cond_bb)?;
                } else {
                    bail!("continue statement outside of loop");
                }
                Ok(())
            }
            Statement::Pass => Ok(()),
            Statement::Struct { .. } => Ok(()), // Handled at module level
            Statement::Enum { .. } => Ok(()),   // Handled at module level
            Statement::TypeAlias { .. } => Ok(()), // Handled at module level
            Statement::Function(_) => Ok(()),   // Handled at module level
            Statement::Use { .. } => Ok(()),    // Handled at module level
            Statement::For {
                var,
                iterable,
                body,
            } => self.lower_for_loop(
                var.as_ref(),
                iterable.as_ref(),
                body.as_ref(),
                function,
                ctx,
            ),
            Statement::PubUse { .. } => Ok(()), // Handled at module level
            Statement::Block(block) => self.lower_block(block.as_ref(), function, ctx),
        }
    }

    fn lower_if_statement(
        &mut self,
        function: FunctionValue<'ctx>,
        ctx: &mut FunctionContext<'ctx>,
        cond: &Expr,
        then_block: &Block,
        elif_blocks: &[(ast::nodes::Node<Expr>, ast::nodes::Node<Block>)],
        else_block: Option<&Block>,
    ) -> Result<()> {
        let cond_val = self.eval_expr(cond, ctx)?;
        let cond_bool = self.to_bool_value(cond_val)?;

        let then_bb = self.context.append_basic_block(function, "then");
        let else_bb = self.context.append_basic_block(function, "else");
        let merge_bb = self.context.append_basic_block(function, "merge");

        self.builder
            .build_conditional_branch(cond_bool, then_bb, else_bb)?;

        // Then block
        self.builder.position_at_end(then_bb);
        self.lower_block(then_block, function, ctx)?;
        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder.build_unconditional_branch(merge_bb)?;
        }

        // Else block (handle elifs recursively or iteratively)
        self.builder.position_at_end(else_bb);
        if !elif_blocks.is_empty() {
            let (next_cond, next_block) = &elif_blocks[0];
            self.lower_if_statement(
                function,
                ctx,
                next_cond.as_ref(),
                next_block.as_ref(),
                &elif_blocks[1..],
                else_block,
            )?;
        } else if let Some(block) = else_block {
            self.lower_block(block, function, ctx)?;
            if self
                .builder
                .get_insert_block()
                .and_then(|b| b.get_terminator())
                .is_none()
            {
                self.builder.build_unconditional_branch(merge_bb)?;
            }
        } else {
            self.builder.build_unconditional_branch(merge_bb)?;
        }

        self.builder.position_at_end(merge_bb);
        Ok(())
    }

    fn lower_while_loop(
        &mut self,
        function: FunctionValue<'ctx>,
        ctx: &mut FunctionContext<'ctx>,
        cond: &Expr,
        body: &Block,
    ) -> Result<()> {
        let cond_bb = self.context.append_basic_block(function, "while_cond");
        let body_bb = self.context.append_basic_block(function, "while_body");
        let exit_bb = self.context.append_basic_block(function, "while_exit");

        self.builder.build_unconditional_branch(cond_bb)?;

        // Condition
        self.builder.position_at_end(cond_bb);
        let cond_val = self.eval_expr(cond, ctx)?;
        let cond_bool = self.to_bool_value(cond_val)?;
        self.builder
            .build_conditional_branch(cond_bool, body_bb, exit_bb)?;

        // Body
        self.builder.position_at_end(body_bb);
        ctx.push_loop(cond_bb, exit_bb);
        self.lower_block(body, function, ctx)?;
        ctx.pop_loop();

        if self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_terminator())
            .is_none()
        {
            self.builder.build_unconditional_branch(cond_bb)?;
        }

        self.builder.position_at_end(exit_bb);
        Ok(())
    }

    fn lower_for_loop(
        &mut self,
        var: &str,
        iterable: &Expr,
        body: &Block,
        function: FunctionValue<'ctx>,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<()> {
        use ast::nodes::Expr;

        // Iterator protocol implementation for range-based for loops

        // Check if iterable is a range expression
        if let Expr::Range { start, end } = iterable {
            // Evaluate start and end
            let start_val = self.eval_expr(start.as_ref().as_ref(), ctx)?;
            let end_val = self.eval_expr(end.as_ref().as_ref(), ctx)?;
            let start_ty = start_val.ty.clone();

            // Determine if we're using I64 or F64 range
            let is_float = start_ty == OtterType::F64;
            
            // Call the appropriate range function to get a list handle
            let range_fn_name = if is_float {
                "range<float>"
            } else {
                "range<int>"
            };
            
            let range_fn = self.get_or_declare_ffi_function(range_fn_name)?;
            
            let list_handle = self.builder.build_call(
                range_fn,
                &[
                    start_val.value.unwrap().into(),
                    end_val.value.unwrap().into(),
                ],
                "range_list",
            )?;
            
            let list_val = list_handle.try_as_basic_value().left().unwrap();
            
            // Now treat it as a list iteration
            let iter_create_fn = self.get_or_declare_ffi_function("__otter_iter_array")?;
            let iter_has_next_fn = self.get_or_declare_ffi_function("__otter_iter_has_next_array")?;
            let iter_next_fn = self.get_or_declare_ffi_function("__otter_iter_next_array")?;
            let iter_free_fn = self.get_or_declare_ffi_function("__otter_iter_free_array")?;

            self.lower_collection_for_loop(
                var,
                EvaluatedValue::with_value(list_val, OtterType::List),
                body,
                function,
                ctx,
                IteratorRuntime {
                    create_fn: iter_create_fn,
                    has_next_fn: iter_has_next_fn,
                    next_fn: iter_next_fn,
                    free_fn: iter_free_fn,
                    element_type: start_ty, // Elements of range have same type as start
                },
            )?;
            
            Ok(())
        } else {
            // Handle other iterable types (arrays, strings, etc.)
            let iterable_val = self.eval_expr(iterable, ctx)?;
            let iterable_ty = iterable_val.ty.clone();

            match iterable_ty {
                OtterType::Str => {
                    // String iteration (character by character)
                    let (iter_create_fn, iter_has_next_fn, iter_next_fn, iter_free_fn) = (
                        self.get_or_declare_ffi_function("__otter_iter_string")?,
                        self.get_or_declare_ffi_function("__otter_iter_has_next_string")?,
                        self.get_or_declare_ffi_function("__otter_iter_next_string")?,
                        self.get_or_declare_ffi_function("__otter_iter_free_string")?,
                    );

                    self.lower_collection_for_loop(
                        var,
                        iterable_val,
                        body,
                        function,
                        ctx,
                        IteratorRuntime {
                            create_fn: iter_create_fn,
                            has_next_fn: iter_has_next_fn,
                            next_fn: iter_next_fn,
                            free_fn: iter_free_fn,
                            element_type: OtterType::Str, // Each character is a string
                        },
                    )
                }
                OtterType::List => {
                    // Array/list iteration
                    let iter_create_fn = self.get_or_declare_ffi_function("__otter_iter_array")?;
                    let iter_has_next_fn =
                        self.get_or_declare_ffi_function("__otter_iter_has_next_array")?;
                    let iter_next_fn =
                        self.get_or_declare_ffi_function("__otter_iter_next_array")?;
                    let iter_free_fn =
                        self.get_or_declare_ffi_function("__otter_iter_free_array")?;

                    let element_type = self
                        .list_element_type(iterable)
                        .unwrap_or(OtterType::Opaque);

                    self.lower_collection_for_loop(
                        var,
                        iterable_val,
                        body,
                        function,
                        ctx,
                        IteratorRuntime {
                            create_fn: iter_create_fn,
                            has_next_fn: iter_has_next_fn,
                            next_fn: iter_next_fn,
                            free_fn: iter_free_fn,
                            element_type,
                        },
                    )
                }
                OtterType::Map => {
                    // Map iteration is not yet implemented
                    bail!("Map iteration is not yet supported")
                }
                _ => bail!(
                    "For loops over type {:?} are not supported yet",
                    iterable_ty
                ),
            }
        }
    }

    fn lower_collection_for_loop(
        &mut self,
        var: &str,
        iterable_val: crate::codegen::llvm::compiler::types::EvaluatedValue<'ctx>,
        body: &Block,
        function: FunctionValue<'ctx>,
        ctx: &mut FunctionContext<'ctx>,
        iter_runtime: IteratorRuntime<'ctx>,
    ) -> Result<()> {
        let IteratorRuntime {
            create_fn,
            has_next_fn,
            next_fn,
            free_fn,
            element_type,
        } = iter_runtime;
        // Create iterator from collection
        let iter_handle = self.builder.build_call(
            create_fn,
            &[iterable_val
                .value
                .ok_or_else(|| anyhow::anyhow!("Iterable has no value"))?
                .into()],
            "iter_handle",
        )?;

        let iter_val = iter_handle
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow::anyhow!("Iterator creation failed"))?;

        // Create loop variable allocation
        let element_ty = element_type.clone();
        let var_alloca = self.create_entry_block_alloca(function, var, element_ty.clone())?;

        // Insert variable into context
        ctx.insert(
            var.to_string(),
            Variable {
                ptr: var_alloca,
                ty: element_ty.clone(),
            },
        );

        // Create basic blocks
        let loop_cond_bb = self.context.append_basic_block(function, "loop_cond");
        let loop_body_bb = self.context.append_basic_block(function, "loop_body");
        let cleanup_bb = self.context.append_basic_block(function, "loop_cleanup");
        let exit_bb = self.context.append_basic_block(function, "loop_exit");

        // Jump to condition check
        self.builder.build_unconditional_branch(loop_cond_bb)?;

        // Condition check block
        self.builder.position_at_end(loop_cond_bb);

        // Check if iterator has next element
        let has_next_call = self
            .builder
            .build_call(has_next_fn, &[iter_val.into()], "has_next")?;

        let has_next = has_next_call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow::anyhow!("has_next check failed"))?
            .into_int_value();

        self.builder
            .build_conditional_branch(has_next, loop_body_bb, cleanup_bb)?;

        // Loop body block
        self.builder.position_at_end(loop_body_bb);

        // Get next element
        let next_call = self
            .builder
            .build_call(next_fn, &[iter_val.into()], "next_element")?;

        let element_val = next_call
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow::anyhow!("next element failed"))?;

        // Decode the runtime type tag and convert to the correct type
        // The runtime now returns tagged values: upper 8 bits = type tag, lower 56 bits = data
        let decoded_value = self.decode_and_convert_tagged_value(element_val, &element_ty)?;
        if let Some(value) = decoded_value {
            self.builder.build_store(var_alloca, value)?;
        }

        // Execute loop body
        ctx.push_loop(loop_cond_bb, cleanup_bb);
        self.lower_block(body, function, ctx)?;
        ctx.pop_loop();

        // Jump back to condition check
        if self.builder.get_insert_block().is_some()
            && self
                .builder
                .get_insert_block()
                .unwrap()
                .get_terminator()
                .is_none()
        {
            self.builder.build_unconditional_branch(loop_cond_bb)?;
        }

        // Cleanup block - free iterator
        self.builder.position_at_end(cleanup_bb);
        self.builder.build_call(free_fn, &[iter_val.into()], "")?;
        self.builder.build_unconditional_branch(exit_bb)?;

        self.builder.position_at_end(exit_bb);

        Ok(())
    }

    // Exception handling (try/except/finally/raise) removed - use Result<T, E> pattern matching instead
    fn list_element_type(&self, iterable: &Expr) -> Option<OtterType> {
        if let Some(ty) = self.expr_type(iterable) {
            self.resolve_list_element_type_from_typeinfo(ty)
        } else {
            None
        }
    }

    fn resolve_list_element_type_from_typeinfo(&self, ty: &TypeInfo) -> Option<OtterType> {
        match ty {
            TypeInfo::List(inner) => self.typeinfo_to_otter_type(inner),
            TypeInfo::Alias { underlying, .. } => {
                self.resolve_list_element_type_from_typeinfo(underlying)
            }
            _ => None,
        }
    }

    pub(crate) fn typeinfo_to_otter_type(&self, ty: &TypeInfo) -> Option<OtterType> {
        match ty {
            TypeInfo::Unit => Some(OtterType::Unit),
            TypeInfo::Bool => Some(OtterType::Bool),
            TypeInfo::I32 => Some(OtterType::I32),
            TypeInfo::I64 => Some(OtterType::I64),
            TypeInfo::F64 => Some(OtterType::F64),
            TypeInfo::Str => Some(OtterType::Str),
            TypeInfo::List(_) => Some(OtterType::List),
            TypeInfo::Dict { .. } => Some(OtterType::Map),
            TypeInfo::Struct { name, .. } => self.struct_id(name).map(OtterType::Struct),
            TypeInfo::Alias { underlying, .. } => self.typeinfo_to_otter_type(underlying),
            _ => None,
        }
    }

    fn decode_and_convert_tagged_value(
        &mut self,
        encoded_value: BasicValueEnum<'ctx>,
        expected_type: &OtterType,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        // Runtime values are encoded with type information in upper 8 bits
        // and either the direct value or a handle to full-precision data in lower 56 bits
        // Type tags: 0=Unit, 1=Bool, 2=I64, 3=F64, 4=String, 5=List, 6=Map
        
        let encoded_int = encoded_value.into_int_value();
        
        // Call the appropriate runtime decode function based on expected type
        match expected_type {
            OtterType::Unit => Ok(None),
            
            OtterType::Bool => {
                let decode_fn = self.get_or_declare_ffi_function("__otter_decode_value_as_bool")?;
                let result = self.builder.build_call(
                    decode_fn,
                    &[encoded_int.into()],
                    "decoded_bool",
                )?;
                Ok(Some(result.try_as_basic_value().left().unwrap()))
            }
            
            OtterType::I64 => {
                let decode_fn = self.get_or_declare_ffi_function("__otter_decode_value_as_i64")?;
                let result = self.builder.build_call(
                    decode_fn,
                    &[encoded_int.into()],
                    "decoded_i64",
                )?;
                Ok(Some(result.try_as_basic_value().left().unwrap()))
            }
            
            OtterType::F64 => {
                let decode_fn = self.get_or_declare_ffi_function("__otter_decode_value_as_f64")?;
                let result = self.builder.build_call(
                    decode_fn,
                    &[encoded_int.into()],
                    "decoded_f64",
                )?;
                Ok(Some(result.try_as_basic_value().left().unwrap()))
            }
            
            OtterType::Str => {
                let decode_fn = self.get_or_declare_ffi_function("__otter_decode_value_as_string")?;
                let result = self.builder.build_call(
                    decode_fn,
                    &[encoded_int.into()],
                    "decoded_string",
                )?;
                Ok(Some(result.try_as_basic_value().left().unwrap()))
            }
            
            OtterType::List | OtterType::Map | OtterType::Opaque => {
                let decode_fn = self.get_or_declare_ffi_function("__otter_decode_value_as_handle")?;
                let result = self.builder.build_call(
                    decode_fn,
                    &[encoded_int.into()],
                    "decoded_handle",
                )?;
                Ok(Some(result.try_as_basic_value().left().unwrap()))
            }
            
            OtterType::I32 => {
                let decode_fn = self.get_or_declare_ffi_function("__otter_decode_value_as_i64")?;
                let result = self.builder.build_call(
                    decode_fn,
                    &[encoded_int.into()],
                    "decoded_i64",
                )?;
                let i64_val = result.try_as_basic_value().left().unwrap().into_int_value();
                Ok(Some(self.builder.build_int_truncate(
                    i64_val,
                    self.context.i32_type(),
                    "truncated_i32",
                )?.into()))
            }
            
            OtterType::Struct(_) | OtterType::Tuple(_) => {
                // WARNING: Structs and tuples are currently handled as opaque handles.
                // If the compiler expects a StructType (by value), this may be incorrect.
                // TODO: Implement proper struct decoding/deserialization from handle.
                let decode_fn = self.get_or_declare_ffi_function("__otter_decode_value_as_handle")?;
                let result = self.builder.build_call(
                    decode_fn,
                    &[encoded_int.into()],
                    "decoded_handle",
                )?;
                Ok(Some(result.try_as_basic_value().left().unwrap()))
            }
        }
    }
    
    #[allow(dead_code)]
    fn prepare_iter_element_for_store(
        &mut self,
        raw_value: BasicValueEnum<'ctx>,
        element_type: &OtterType,
    ) -> Result<Option<BasicValueEnum<'ctx>>> {
        let value = match element_type {
            OtterType::Unit => return Ok(None),
            OtterType::I64 | OtterType::Opaque | OtterType::List | OtterType::Map => raw_value,
            OtterType::Struct(_) | OtterType::Tuple(_) => raw_value,
            OtterType::I32 => {
                let int_val = raw_value.into_int_value();
                self.builder
                    .build_int_truncate(int_val, self.context.i32_type(), "iter_i32")?
                    .into()
            }
            OtterType::F64 => {
                // raw_value is an i64 containing the bit pattern of an f64
                // We need to bitcast i64 -> f64
                let int_val = raw_value.into_int_value();
                self.builder
                    .build_bit_cast(int_val, self.context.f64_type(), "iter_f64")?
            }
            OtterType::Bool => {
                let int_val = raw_value.into_int_value();
                self.builder
                    .build_int_truncate(int_val, self.context.bool_type(), "iter_bool")?
                    .into()
            }
            OtterType::Str => {
                let int_val = raw_value.into_int_value();
                self.builder
                    .build_int_to_ptr(int_val, self.string_ptr_type, "iter_str")?
                    .into()
            }
        };

        Ok(Some(value))
    }
}
