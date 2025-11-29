use anyhow::{Result, anyhow, bail};
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, IntValue};

use crate::codegen::llvm::compiler::Compiler;
use crate::codegen::llvm::compiler::types::{EvaluatedValue, FunctionContext, OtterType};
use crate::typecheck::TypeInfo;
use ast::nodes::{BinaryOp, Block, Expr, Literal, Node, Statement, UnaryOp};

impl<'ctx> Compiler<'ctx> {
    pub(crate) fn eval_expr(
        &mut self,
        expr: &Expr,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        match expr {
            Expr::Literal(lit) => self.eval_literal(lit.as_ref()),
            Expr::Identifier(name) => {
                if let Some(var) = ctx.get(name) {
                    let var_ty = var.ty.clone();
                    if let Some(basic_ty) = self.basic_type(var_ty.clone())? {
                        let val = self.builder.build_load(basic_ty, var.ptr, name)?;
                        Ok(EvaluatedValue::with_value(val, var_ty))
                    } else {
                        // Unit type - no value to load
                        Ok(EvaluatedValue {
                            ty: var_ty,
                            value: None,
                        })
                    }
                } else {
                    bail!("Variable {} not found", name);
                }
            }
            Expr::Binary { left, op, right } => {
                self.eval_binary_expr(left.as_ref().as_ref(), op, right.as_ref().as_ref(), ctx)
            }
            Expr::Unary { op, expr } => self.eval_unary_expr(op, expr.as_ref().as_ref(), ctx),
            Expr::Call { func: _, args: _ } => self.eval_call_expr(expr, ctx),
            Expr::Member { object, field } => {
                if let Some(value) =
                    self.try_build_enum_member(expr, object.as_ref().as_ref(), field, ctx)?
                {
                    Ok(value)
                } else if self
                    .module_path_from_expr(object.as_ref().as_ref())
                    .is_some()
                {
                    // Module references don't need runtime materialization.
                    Ok(EvaluatedValue {
                        ty: OtterType::Unit,
                        value: None,
                    })
                } else {
                    let object_value = self.eval_expr(object.as_ref().as_ref(), ctx)?;
                    if object_value.value.is_none() {
                        bail!("cannot access field '{}' without value", field);
                    }
                    let object_ty = object_value.ty.clone();
                    if let OtterType::Struct(struct_id) = object_ty {
                        let struct_value = object_value.value.unwrap().into_struct_value();
                        let info = self.struct_info(struct_id);
                        let idx = info.field_indices.get(field).copied().ok_or_else(|| {
                            anyhow!("struct '{}' has no field '{}'", info.name, field)
                        })?;
                        let extracted = self
                            .builder
                            .build_extract_value(struct_value, idx as u32, field)
                            .map_err(|e| anyhow!("failed to extract field '{}': {e}", field))?;
                        let field_ty = info.field_types[idx].clone();
                        Ok(EvaluatedValue::with_value(extracted, field_ty))
                    } else {
                        bail!(
                            "Complex member expressions not yet supported (expr, ty={:?})",
                            object_ty
                        );
                    }
                }
            }
            Expr::Struct { name, fields } => {
                let (struct_id, _) = self
                    .struct_info_by_name(name)
                    .ok_or_else(|| anyhow!("unknown struct type '{}'", name))?;
                let (struct_name, struct_ty) = {
                    let info = self.struct_info(struct_id);
                    (info.name.clone(), info.ty)
                };
                let mut aggregate = struct_ty.get_undef();
                for (field_name, field_expr) in fields {
                    let idx = {
                        let info = self.struct_info(struct_id);
                        info.field_indices.get(field_name).copied().ok_or_else(|| {
                            anyhow!("struct '{}' has no field '{}'", struct_name, field_name)
                        })?
                    };
                    let field_value = self.eval_expr(field_expr.as_ref(), ctx)?;
                    let raw_value = field_value
                        .value
                        .ok_or_else(|| anyhow!("field '{}' produced no value", field_name))?;
                    let expected_ty = {
                        let info = self.struct_info(struct_id);
                        info.field_types[idx].clone()
                    };
                    let coerced =
                        self.coerce_type(raw_value, field_value.ty.clone(), expected_ty)?;
                    aggregate = self
                        .builder
                        .build_insert_value(aggregate, coerced, idx as u32, field_name)
                        .map_err(|e| anyhow!("failed to insert field '{}': {e}", field_name))?
                        .into_struct_value();
                }

                Ok(EvaluatedValue::with_value(
                    aggregate.into(),
                    OtterType::Struct(struct_id),
                ))
            }
            Expr::If {
                cond: _,
                then_branch: _,
                else_branch: _,
            } => self.eval_if_expr(expr, ctx),
            Expr::Match { value: _, arms: _ } => self.eval_match_expr(expr, ctx),
            Expr::FString { parts: _ } => self.eval_fstring_expr(expr, ctx),
            Expr::Array(elements) => self.eval_array_expr(elements, ctx),
            _ => bail!("Expression type not implemented: {:?}", expr),
        }
    }

    fn eval_match_expr(
        &mut self,
        expr: &Expr,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        let Expr::Match { value, arms } = expr else {
            bail!("Expected Match expression")
        };

        let matched_val = self.eval_expr(value.as_ref().as_ref(), ctx)?;
        let function = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();
        let merge_bb = self.context.append_basic_block(function, "match_merge");

        let mut incoming_values = Vec::new();
        let mut incoming_blocks = Vec::new();

        let mut next_check_bb = self
            .context
            .append_basic_block(function, "match_arm_check_0");
        self.builder.build_unconditional_branch(next_check_bb)?;

        for (i, arm) in arms.iter().enumerate() {
            self.builder.position_at_end(next_check_bb);

            next_check_bb = if i < arms.len() - 1 {
                self.context
                    .append_basic_block(function, &format!("match_arm_check_{}", i + 1))
            } else {
                self.context.append_basic_block(function, "match_no_match")
            };

            let body_bb = self
                .context
                .append_basic_block(function, &format!("match_arm_body_{}", i));

            self.compile_pattern_match(
                &arm.as_ref().pattern,
                &matched_val,
                body_bb,
                next_check_bb,
                ctx,
            )?;

            self.builder.position_at_end(body_bb);
            let body_val = self.lower_block_expression(&arm.as_ref().body, function, ctx)?;

            if self
                .builder
                .get_insert_block()
                .unwrap()
                .get_terminator()
                .is_none()
            {
                self.builder.build_unconditional_branch(merge_bb)?;
            }

            let current_bb = self.builder.get_insert_block().unwrap();
            if let Some(v) = body_val.value {
                incoming_values.push(v);
                incoming_blocks.push(current_bb);
            }
        }

        self.builder.position_at_end(next_check_bb);
        self.builder.build_unreachable()?;

        self.builder.position_at_end(merge_bb);

        if !incoming_values.is_empty() {
            let phi_type = incoming_values[0].get_type();
            let phi = self.builder.build_phi(phi_type, "match_result")?;

            for (val, block) in incoming_values.iter().zip(incoming_blocks.iter()) {
                phi.add_incoming(&[(val, *block)]);
            }

            // Infer the result type from the match expression's type annotation if available
            let match_expr_id = expr as *const Expr as usize;
            let result_ty = if let Some(type_info) = self.expr_types.get(&match_expr_id) {
                self.typeinfo_to_otter_type(type_info).unwrap_or_else(|| {
                    // Fallback to inferring from phi type
                    if phi_type.is_pointer_type() {
                        OtterType::Str
                    } else if phi_type.is_int_type() {
                        let int_type = phi_type.into_int_type();
                        if int_type.get_bit_width() == 64 {
                            OtterType::I64
                        } else if int_type.get_bit_width() == 1 {
                            OtterType::Bool
                        } else {
                            OtterType::I32
                        }
                    } else if phi_type.is_float_type() {
                        OtterType::F64
                    } else {
                        OtterType::Opaque
                    }
                })
            } else {
                // No type info, infer from phi type
                if phi_type.is_pointer_type() {
                    OtterType::Str
                } else if phi_type.is_int_type() {
                    let int_type = phi_type.into_int_type();
                    if int_type.get_bit_width() == 64 {
                        OtterType::I64
                    } else if int_type.get_bit_width() == 1 {
                        OtterType::Bool
                    } else {
                        OtterType::I32
                    }
                } else if phi_type.is_float_type() {
                    OtterType::F64
                } else {
                    OtterType::Opaque
                }
            };

            Ok(EvaluatedValue::with_value(
                phi.as_basic_value(),
                result_ty,
            ))
        } else {
            Ok(EvaluatedValue {
                ty: OtterType::Unit,
                value: None,
            })
        }
    }

    fn compile_pattern_match(
        &mut self,
        pattern: &Node<ast::nodes::Pattern>,
        matched_val: &EvaluatedValue<'ctx>,
        success_bb: inkwell::basic_block::BasicBlock<'ctx>,
        fail_bb: inkwell::basic_block::BasicBlock<'ctx>,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<()> {
        use ast::nodes::Pattern;

        match pattern.as_ref() {
            Pattern::Wildcard => {
                self.builder.build_unconditional_branch(success_bb)?;
                Ok(())
            }
            Pattern::Literal(lit) => {
                let lit_val = self.eval_literal(lit.as_ref())?;
                let is_equal = self.build_equality_check(matched_val, &lit_val)?;
                self.builder
                    .build_conditional_branch(is_equal, success_bb, fail_bb)?;
                Ok(())
            }
            Pattern::Identifier(name) => {
                let function = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let matched_ty = matched_val.ty.clone();
                let alloca = self.create_entry_block_alloca(function, name, matched_ty.clone())?;

                if let Some(v) = matched_val.value {
                    self.builder.build_store(alloca, v)?;
                }

                ctx.insert(
                    name.clone(),
                    crate::codegen::llvm::compiler::types::Variable {
                        ptr: alloca,
                        ty: matched_ty,
                    },
                );

                self.builder.build_unconditional_branch(success_bb)?;
                Ok(())
            }
            Pattern::EnumVariant {
                enum_name,
                variant,
                fields,
            } => {
                let get_tag_fn = self.get_or_declare_ffi_function("runtime.enum.get_tag")?;
                let handle = matched_val
                    .value
                    .ok_or_else(|| anyhow!("Enum value is void"))?;

                let tag_val = self
                    .builder
                    .build_call(get_tag_fn, &[handle.into()], "tag")?
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                let layout = self
                    .enum_layout(enum_name)
                    .ok_or_else(|| anyhow!("Enum layout not found for {}", enum_name))?;
                let expected_tag = layout
                    .tag_of(variant)
                    .ok_or_else(|| anyhow!("Variant {} not found in {}", variant, enum_name))?;

                let expected_tag_val = self
                    .context
                    .i64_type()
                    .const_int(expected_tag as u64, false);
                let tag_match = self.builder.build_int_compare(
                    IntPredicate::EQ,
                    tag_val,
                    expected_tag_val,
                    "tag_match",
                )?;

                if fields.is_empty() {
                    self.builder
                        .build_conditional_branch(tag_match, success_bb, fail_bb)?;
                } else {
                    let field_check_bb = self.context.append_basic_block(
                        self.builder
                            .get_insert_block()
                            .unwrap()
                            .get_parent()
                            .unwrap(),
                        "enum_field_check",
                    );
                    self.builder
                        .build_conditional_branch(tag_match, field_check_bb, fail_bb)?;
                    self.builder.position_at_end(field_check_bb);

                    // Get enum type to determine field types
                    let enum_type = self
                        .enum_type(enum_name)
                        .ok_or_else(|| anyhow!("Enum type not found for {}", enum_name))?
                        .clone();
                    let TypeInfo::Enum { variants, .. } = enum_type else {
                        bail!("Expected enum type for {}", enum_name);
                    };
                    let variant_info = variants
                        .get(variant)
                        .ok_or_else(|| {
                            anyhow!("Variant {} not found in enum {}", variant, enum_name)
                        })?
                        .clone();

                    for (field_idx, field_pattern) in fields.iter().enumerate() {
                        let field_type = variant_info
                            .fields
                            .get(field_idx)
                            .ok_or_else(|| {
                                anyhow!(
                                    "Field index {} out of bounds for variant {}",
                                    field_idx,
                                    variant
                                )
                            })?
                            .clone();

                        // Use type-specific getter based on field type
                        let get_field_fn_name = match enum_field_kind(&field_type) {
                            EnumFieldKind::Int => "runtime.enum.get_i64",
                            EnumFieldKind::Float => "runtime.enum.get_f64",
                            EnumFieldKind::Bool => "runtime.enum.get_bool",
                            EnumFieldKind::Ptr => "runtime.enum.get_ptr",
                        };
                        let get_field_fn = self.get_or_declare_ffi_function(get_field_fn_name)?;
                        let field_idx_val =
                            self.context.i64_type().const_int(field_idx as u64, false);
                        let field_val = self
                            .builder
                            .build_call(
                                get_field_fn,
                                &[handle.into(), field_idx_val.into()],
                                "field",
                            )?
                            .try_as_basic_value()
                            .left()
                            .unwrap();

                        // Convert to appropriate OtterType
                        let field_otter_type = match enum_field_kind(&field_type) {
                            EnumFieldKind::Int => OtterType::I64,
                            EnumFieldKind::Float => OtterType::F64,
                            EnumFieldKind::Bool => OtterType::Bool,
                            EnumFieldKind::Ptr => OtterType::Opaque,
                        };
                        let field_eval = EvaluatedValue::with_value(field_val, field_otter_type);

                        let next_field_bb = if field_idx < fields.len() - 1 {
                            self.context.append_basic_block(
                                self.builder
                                    .get_insert_block()
                                    .unwrap()
                                    .get_parent()
                                    .unwrap(),
                                &format!("enum_field_check_{}", field_idx + 1),
                            )
                        } else {
                            success_bb
                        };

                        self.compile_pattern_match(
                            field_pattern,
                            &field_eval,
                            next_field_bb,
                            fail_bb,
                            ctx,
                        )?;

                        if field_idx < fields.len() - 1 {
                            self.builder.position_at_end(next_field_bb);
                        }
                    }
                }

                Ok(())
            }
            Pattern::Struct {
                name: _struct_name,
                fields,
            } => {
                if fields.is_empty() {
                    self.builder.build_unconditional_branch(success_bb)?;
                } else {
                    bail!("Struct pattern matching with fields not yet fully supported");
                }
                Ok(())
            }
            Pattern::Array { patterns, rest } => {
                let get_len_fn = self.get_or_declare_ffi_function("runtime.list.length")?;
                let handle = matched_val
                    .value
                    .ok_or_else(|| anyhow!("Array value is void"))?;
                let len_val = self
                    .builder
                    .build_call(get_len_fn, &[handle.into()], "len")?
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_int_value();

                let min_len = patterns.len() as u64;

                let expected_len = self.context.i64_type().const_int(min_len, false);
                let len_check = if rest.is_some() {
                    self.builder.build_int_compare(
                        IntPredicate::UGE,
                        len_val,
                        expected_len,
                        "len_check",
                    )?
                } else {
                    self.builder.build_int_compare(
                        IntPredicate::EQ,
                        len_val,
                        expected_len,
                        "len_check",
                    )?
                };

                let elem_check_bb = self.context.append_basic_block(
                    self.builder
                        .get_insert_block()
                        .unwrap()
                        .get_parent()
                        .unwrap(),
                    "array_elem_check",
                );
                self.builder
                    .build_conditional_branch(len_check, elem_check_bb, fail_bb)?;
                self.builder.position_at_end(elem_check_bb);

                for (idx, elem_pattern) in patterns.iter().enumerate() {
                    let get_elem_fn = self.get_or_declare_ffi_function("runtime.list.get")?;
                    let idx_val = self.context.i64_type().const_int(idx as u64, false);
                    let elem_val = self
                        .builder
                        .build_call(get_elem_fn, &[handle.into(), idx_val.into()], "elem")?
                        .try_as_basic_value()
                        .left()
                        .unwrap();

                    let elem_eval = EvaluatedValue::with_value(elem_val, OtterType::Opaque);

                    let next_bb = if idx < patterns.len() - 1 || rest.is_some() {
                        self.context.append_basic_block(
                            self.builder
                                .get_insert_block()
                                .unwrap()
                                .get_parent()
                                .unwrap(),
                            &format!("array_elem_check_{}", idx + 1),
                        )
                    } else {
                        success_bb
                    };

                    self.compile_pattern_match(elem_pattern, &elem_eval, next_bb, fail_bb, ctx)?;

                    if idx < patterns.len() - 1 || rest.is_some() {
                        self.builder.position_at_end(next_bb);
                    }
                }

                if let Some(rest_name) = rest {
                    let function = self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_parent()
                        .unwrap();
                    let alloca =
                        self.create_entry_block_alloca(function, rest_name, OtterType::List)?;
                    self.builder.build_store(alloca, handle)?;
                    ctx.insert(
                        rest_name.clone(),
                        crate::codegen::llvm::compiler::types::Variable {
                            ptr: alloca,
                            ty: OtterType::List,
                        },
                    );
                    self.builder.build_unconditional_branch(success_bb)?;
                }

                Ok(())
            }
        }
    }

    fn build_equality_check(
        &mut self,
        lhs: &EvaluatedValue<'ctx>,
        rhs: &EvaluatedValue<'ctx>,
    ) -> Result<IntValue<'ctx>> {
        let lhs_ty = lhs.ty.clone();
        match lhs_ty {
            OtterType::I64 => {
                let l = lhs.value.unwrap().into_int_value();
                let r = rhs.value.unwrap().into_int_value();
                Ok(self
                    .builder
                    .build_int_compare(IntPredicate::EQ, l, r, "eq")?)
            }
            OtterType::F64 => {
                let l = lhs.value.unwrap().into_float_value();
                let r = rhs.value.unwrap().into_float_value();
                Ok(self
                    .builder
                    .build_float_compare(inkwell::FloatPredicate::OEQ, l, r, "eq")?)
            }
            OtterType::Bool => {
                let l = lhs.value.unwrap().into_int_value();
                let r = rhs.value.unwrap().into_int_value();
                Ok(self
                    .builder
                    .build_int_compare(IntPredicate::EQ, l, r, "eq")?)
            }
            OtterType::Str => {
                let left_ptr = self.ensure_string_value(lhs.clone())?;
                let right_ptr = self.ensure_string_value(rhs.clone())?;
                let eq_fn = self.get_or_declare_ffi_function("std.strings.equal")?;
                let res_call = self
                    .builder
                    .build_call(eq_fn, &[left_ptr.into(), right_ptr.into()], "str_eq")?
                    .try_as_basic_value()
                    .left()
                    .unwrap();
                // Function returns i32, convert to i1 (bool) for comparison
                let res_i32 = res_call.into_int_value();
                // Check if it's i32 or i64 and convert accordingly
                let zero = if res_i32.get_type().get_bit_width() == 32 {
                    self.context.i32_type().const_zero()
                } else {
                    self.context.i64_type().const_zero()
                };
                let res_bool = self.builder.build_int_compare(
                    IntPredicate::NE,
                    res_i32,
                    zero,
                    "str_eq_bool",
                )?;
                Ok(res_bool)
            }
            _ => bail!("Equality check not implemented for type {:?}", lhs_ty),
        }
    }

    fn lower_block_expression(
        &mut self,
        block: &Node<Block>,
        function: FunctionValue<'ctx>,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        for stmt in &block.as_ref().statements {
            if let Statement::Expr(e) = stmt.as_ref() {
                if std::ptr::eq(stmt, block.as_ref().statements.last().unwrap()) {
                    return self.eval_expr(e.as_ref(), ctx);
                } else {
                    self.eval_expr(e.as_ref(), ctx)?;
                }
            } else {
                self.lower_statement(stmt.as_ref(), function, ctx)?;
            }
        }

        Ok(EvaluatedValue {
            ty: OtterType::Unit,
            value: None,
        })
    }

    fn eval_fstring_expr(
        &mut self,
        expr: &Expr,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        let Expr::FString { parts } = expr else {
            bail!("Expected FString expression")
        };

        let mut result = self.eval_literal(&Literal::String("".to_string()))?;

        for part in parts {
            let part_val = match part.as_ref() {
                ast::nodes::FStringPart::Text(s) => {
                    self.eval_literal(&Literal::String(s.clone()))?
                }
                ast::nodes::FStringPart::Expr(e) => self.eval_expr(e.as_ref(), ctx)?,
            };

            result = self.build_string_concat(result, part_val)?;
        }

        Ok(result)
    }

    fn eval_literal(&mut self, lit: &Literal) -> Result<EvaluatedValue<'ctx>> {
        match lit {
            Literal::Number(n) => {
                let val = self.context.f64_type().const_float(n.value);
                Ok(EvaluatedValue::with_value(val.into(), OtterType::F64))
            }
            Literal::String(s) => {
                let val = self.builder.build_global_string_ptr(s, "str_lit")?;
                Ok(EvaluatedValue::with_value(
                    val.as_pointer_value().into(),
                    OtterType::Str,
                ))
            }
            Literal::Bool(b) => {
                let val = self.context.bool_type().const_int(*b as u64, false);
                Ok(EvaluatedValue::with_value(val.into(), OtterType::Bool))
            }
            Literal::Unit => Ok(EvaluatedValue {
                ty: OtterType::Unit,
                value: None,
            }),
            Literal::None => Ok(EvaluatedValue {
                ty: OtterType::Unit,
                value: None,
            }), // Treat None as Unit for now
        }
    }

    fn eval_binary_expr(
        &mut self,
        left: &Expr,
        op: &BinaryOp,
        right: &Expr,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        let lhs = self.eval_expr(left, ctx)?;
        let rhs = self.eval_expr(right, ctx)?;
        let lhs_ty = lhs.ty.clone();
        let rhs_ty = rhs.ty.clone();

        if matches!(op, BinaryOp::Add) && (lhs_ty == OtterType::Str || rhs_ty == OtterType::Str) {
            return self.build_string_concat(lhs, rhs);
        }

        if lhs_ty == OtterType::Str && rhs_ty == OtterType::Str {
            return match op {
                BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Lt
                | BinaryOp::Gt
                | BinaryOp::LtEq
                | BinaryOp::GtEq => {
                    let is_equal = self.build_equality_check(&lhs, &rhs)?;
                    match op {
                        BinaryOp::Eq => {
                            Ok(EvaluatedValue::with_value(is_equal.into(), OtterType::Bool))
                        }
                        BinaryOp::Ne => {
                            let not_equal = self.builder.build_not(is_equal, "ne")?;
                            Ok(EvaluatedValue::with_value(
                                not_equal.into(),
                                OtterType::Bool,
                            ))
                        }
                        _ => {
                            let cmp_fn = self.get_or_declare_ffi_function("std.strings.compare")?;
                            let left_ptr = self.ensure_string_value(lhs.clone())?;
                            let right_ptr = self.ensure_string_value(rhs.clone())?;
                            let cmp_result = self
                                .builder
                                .build_call(cmp_fn, &[left_ptr.into(), right_ptr.into()], "strcmp")?
                                .try_as_basic_value()
                                .left()
                                .unwrap()
                                .into_int_value();
                            let zero = self.context.i64_type().const_zero();

                            let result = match op {
                                BinaryOp::Lt => self.builder.build_int_compare(
                                    IntPredicate::SLT,
                                    cmp_result,
                                    zero,
                                    "lt",
                                )?,
                                BinaryOp::Gt => self.builder.build_int_compare(
                                    IntPredicate::SGT,
                                    cmp_result,
                                    zero,
                                    "gt",
                                )?,
                                BinaryOp::LtEq => self.builder.build_int_compare(
                                    IntPredicate::SLE,
                                    cmp_result,
                                    zero,
                                    "le",
                                )?,
                                BinaryOp::GtEq => self.builder.build_int_compare(
                                    IntPredicate::SGE,
                                    cmp_result,
                                    zero,
                                    "ge",
                                )?,
                                _ => unreachable!(),
                            };
                            Ok(EvaluatedValue::with_value(result.into(), OtterType::Bool))
                        }
                    }
                }
                _ => bail!("Unsupported binary operation for strings: {:?}", op),
            };
        }

        // Coerce types if needed - promote to F64 if either operand is F64
        let (lhs_val, rhs_val, result_ty) = if lhs_ty == OtterType::F64 || rhs_ty == OtterType::F64
        {
            // Promote both to F64
            let l_f64 = if lhs_ty == OtterType::F64 {
                lhs.value.unwrap().into_float_value()
            } else if lhs_ty == OtterType::I64 {
                let int_val = lhs.value.unwrap().into_int_value();
                self.builder
                    .build_signed_int_to_float(int_val, self.context.f64_type(), "itof")?
            } else if lhs_ty == OtterType::Opaque {
                let coerced = self.coerce_type(lhs.value.unwrap(), lhs_ty, OtterType::F64)?;
                coerced.into_float_value()
            } else {
                bail!("Cannot coerce {:?} to F64", lhs_ty);
            };

            let r_f64 = if rhs_ty == OtterType::F64 {
                rhs.value.unwrap().into_float_value()
            } else if rhs_ty == OtterType::I64 {
                let int_val = rhs.value.unwrap().into_int_value();
                self.builder
                    .build_signed_int_to_float(int_val, self.context.f64_type(), "itof")?
            } else if rhs_ty == OtterType::Opaque {
                let coerced = self.coerce_type(rhs.value.unwrap(), rhs_ty, OtterType::F64)?;
                coerced.into_float_value()
            } else {
                bail!("Cannot coerce {:?} to F64", rhs_ty);
            };

            (l_f64.into(), r_f64.into(), OtterType::F64)
        } else if lhs_ty == OtterType::I64 && rhs_ty == OtterType::I64 {
            (lhs.value.unwrap(), rhs.value.unwrap(), OtterType::I64)
        } else if lhs_ty == OtterType::Bool && rhs_ty == OtterType::Bool {
            (lhs.value.unwrap(), rhs.value.unwrap(), OtterType::Bool)
        } else {
            bail!(
                "Type mismatch or unsupported types for binary op: {:?} and {:?}",
                lhs_ty,
                rhs_ty
            );
        };

        // Perform the operation based on the result type
        match result_ty {
            OtterType::I64 => {
                let l = lhs_val.into_int_value();
                let r = rhs_val.into_int_value();
                match op {
                    BinaryOp::Add => Ok(EvaluatedValue::with_value(
                        self.builder.build_int_add(l, r, "add")?.into(),
                        OtterType::I64,
                    )),
                    BinaryOp::Sub => Ok(EvaluatedValue::with_value(
                        self.builder.build_int_sub(l, r, "sub")?.into(),
                        OtterType::I64,
                    )),
                    BinaryOp::Mul => Ok(EvaluatedValue::with_value(
                        self.builder.build_int_mul(l, r, "mul")?.into(),
                        OtterType::I64,
                    )),
                    BinaryOp::Div => Ok(EvaluatedValue::with_value(
                        self.builder.build_int_signed_div(l, r, "div")?.into(),
                        OtterType::I64,
                    )),
                    BinaryOp::Eq => Ok(EvaluatedValue::with_value(
                        self.builder
                            .build_int_compare(IntPredicate::EQ, l, r, "eq")?
                            .into(),
                        OtterType::Bool,
                    )),
                    BinaryOp::Ne => Ok(EvaluatedValue::with_value(
                        self.builder
                            .build_int_compare(IntPredicate::NE, l, r, "ne")?
                            .into(),
                        OtterType::Bool,
                    )),
                    BinaryOp::Lt => Ok(EvaluatedValue::with_value(
                        self.builder
                            .build_int_compare(IntPredicate::SLT, l, r, "lt")?
                            .into(),
                        OtterType::Bool,
                    )),
                    BinaryOp::Gt => Ok(EvaluatedValue::with_value(
                        self.builder
                            .build_int_compare(IntPredicate::SGT, l, r, "gt")?
                            .into(),
                        OtterType::Bool,
                    )),
                    BinaryOp::LtEq => Ok(EvaluatedValue::with_value(
                        self.builder
                            .build_int_compare(IntPredicate::SLE, l, r, "le")?
                            .into(),
                        OtterType::Bool,
                    )),
                    BinaryOp::GtEq => Ok(EvaluatedValue::with_value(
                        self.builder
                            .build_int_compare(IntPredicate::SGE, l, r, "ge")?
                            .into(),
                        OtterType::Bool,
                    )),
                    _ => bail!("Unsupported binary op for I64"),
                }
            }
            OtterType::F64 => {
                let l = lhs_val.into_float_value();
                let r = rhs_val.into_float_value();
                match op {
                    BinaryOp::Add => Ok(EvaluatedValue::with_value(
                        self.builder.build_float_add(l, r, "add")?.into(),
                        OtterType::F64,
                    )),
                    BinaryOp::Sub => Ok(EvaluatedValue::with_value(
                        self.builder.build_float_sub(l, r, "sub")?.into(),
                        OtterType::F64,
                    )),
                    BinaryOp::Mul => Ok(EvaluatedValue::with_value(
                        self.builder.build_float_mul(l, r, "mul")?.into(),
                        OtterType::F64,
                    )),
                    BinaryOp::Div => Ok(EvaluatedValue::with_value(
                        self.builder.build_float_div(l, r, "div")?.into(),
                        OtterType::F64,
                    )),
                    BinaryOp::Eq => Ok(EvaluatedValue::with_value(
                        self.builder
                            .build_float_compare(inkwell::FloatPredicate::OEQ, l, r, "eq")?
                            .into(),
                        OtterType::Bool,
                    )),
                    BinaryOp::Ne => Ok(EvaluatedValue::with_value(
                        self.builder
                            .build_float_compare(inkwell::FloatPredicate::ONE, l, r, "ne")?
                            .into(),
                        OtterType::Bool,
                    )),
                    BinaryOp::Lt => Ok(EvaluatedValue::with_value(
                        self.builder
                            .build_float_compare(inkwell::FloatPredicate::OLT, l, r, "lt")?
                            .into(),
                        OtterType::Bool,
                    )),
                    BinaryOp::Gt => Ok(EvaluatedValue::with_value(
                        self.builder
                            .build_float_compare(inkwell::FloatPredicate::OGT, l, r, "gt")?
                            .into(),
                        OtterType::Bool,
                    )),
                    BinaryOp::LtEq => Ok(EvaluatedValue::with_value(
                        self.builder
                            .build_float_compare(inkwell::FloatPredicate::OLE, l, r, "le")?
                            .into(),
                        OtterType::Bool,
                    )),
                    BinaryOp::GtEq => Ok(EvaluatedValue::with_value(
                        self.builder
                            .build_float_compare(inkwell::FloatPredicate::OGE, l, r, "ge")?
                            .into(),
                        OtterType::Bool,
                    )),
                    _ => bail!("Unsupported binary op for F64"),
                }
            }
            _ => bail!("Unsupported type for binary operation"),
        }
    }

    fn eval_unary_expr(
        &mut self,
        op: &UnaryOp,
        expr: &Expr,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        let val = self.eval_expr(expr, ctx)?;
        match op {
            UnaryOp::Neg => {
                let val_ty = val.ty.clone();
                if val_ty == OtterType::I64 {
                    let v = val.value.unwrap().into_int_value();
                    Ok(EvaluatedValue::with_value(
                        self.builder.build_int_neg(v, "neg")?.into(),
                        OtterType::I64,
                    ))
                } else if val_ty == OtterType::F64 {
                    let v = val.value.unwrap().into_float_value();
                    Ok(EvaluatedValue::with_value(
                        self.builder.build_float_neg(v, "neg")?.into(),
                        OtterType::F64,
                    ))
                } else {
                    bail!("Unsupported type for negation");
                }
            }
            UnaryOp::Not => {
                let val_ty = val.ty.clone();
                if val_ty == OtterType::Bool {
                    let v = val.value.unwrap().into_int_value();
                    Ok(EvaluatedValue::with_value(
                        self.builder.build_not(v, "not")?.into(),
                        OtterType::Bool,
                    ))
                } else {
                    bail!("Unsupported type for not");
                }
            }
        }
    }

    pub(crate) fn basic_type(&self, ty: OtterType) -> Result<Option<BasicTypeEnum<'ctx>>> {
        match ty {
            OtterType::Unit => Ok(None),
            OtterType::Bool => Ok(Some(self.context.bool_type().into())),
            OtterType::I32 => Ok(Some(self.context.i32_type().into())),
            OtterType::I64 => Ok(Some(self.context.i64_type().into())),
            OtterType::F64 => Ok(Some(self.context.f64_type().into())),
            OtterType::Str => Ok(Some(self.string_ptr_type.into())),
            OtterType::Opaque => Ok(Some(self.context.i64_type().into())),
            OtterType::List => Ok(Some(self.context.i64_type().into())),
            OtterType::Map => Ok(Some(self.context.i64_type().into())),
            OtterType::Struct(id) => Ok(Some(self.struct_info(id).ty.into())),
            OtterType::Tuple(fields) => {
                let mut llvm_fields = Vec::with_capacity(fields.len());
                for field in fields {
                    let llvm_ty = match self.basic_type(field)? {
                        Some(ty) => ty,
                        None => self.context.i8_type().into(),
                    };
                    llvm_fields.push(llvm_ty);
                }
                Ok(Some(self.context.struct_type(&llvm_fields, false).into()))
            }
        }
    }

    pub(crate) fn to_bool_value(&self, val: EvaluatedValue<'ctx>) -> Result<IntValue<'ctx>> {
        let EvaluatedValue { ty, value } = val;
        if ty == OtterType::Bool {
            Ok(value.unwrap().into_int_value())
        } else {
            bail!("Expected boolean value")
        }
    }

    pub(crate) fn coerce_type(
        &self,
        value: inkwell::values::BasicValueEnum<'ctx>,
        from_ty: OtterType,
        to_ty: OtterType,
    ) -> Result<inkwell::values::BasicValueEnum<'ctx>> {
        // If types match, no coercion needed
        if from_ty == to_ty {
            return Ok(value);
        }

        // Perform type coercion based on source and target types
        match (from_ty.clone(), to_ty.clone()) {
            // Numeric conversions
            (OtterType::I32, OtterType::I64) => {
                let int_val = value.into_int_value();
                Ok(self
                    .builder
                    .build_int_s_extend(int_val, self.context.i64_type(), "i32_to_i64")?
                    .into())
            }
            (OtterType::I64, OtterType::I32) => {
                let int_val = value.into_int_value();
                Ok(self
                    .builder
                    .build_int_truncate(int_val, self.context.i32_type(), "i64_to_i32")?
                    .into())
            }
            (OtterType::I32, OtterType::F64) | (OtterType::I64, OtterType::F64) => {
                let int_val = value.into_int_value();
                Ok(self
                    .builder
                    .build_signed_int_to_float(int_val, self.context.f64_type(), "int_to_f64")?
                    .into())
            }
            (OtterType::F64, OtterType::I32) => {
                let float_val = value.into_float_value();
                Ok(self
                    .builder
                    .build_float_to_signed_int(float_val, self.context.i32_type(), "f64_to_i32")?
                    .into())
            }
            (OtterType::F64, OtterType::I64) => {
                let float_val = value.into_float_value();
                Ok(self
                    .builder
                    .build_float_to_signed_int(float_val, self.context.i64_type(), "f64_to_i64")?
                    .into())
            }

            // Bool conversions
            (OtterType::Bool, OtterType::I32) => {
                let bool_val = value.into_int_value();
                Ok(self
                    .builder
                    .build_int_z_extend(bool_val, self.context.i32_type(), "bool_to_i32")?
                    .into())
            }
            (OtterType::Bool, OtterType::I64) => {
                let bool_val = value.into_int_value();
                Ok(self
                    .builder
                    .build_int_z_extend(bool_val, self.context.i64_type(), "bool_to_i64")?
                    .into())
            }
            (OtterType::I32 | OtterType::I64, OtterType::Bool) => {
                let int_val = value.into_int_value();
                let zero = int_val.get_type().const_zero();
                Ok(self
                    .builder
                    .build_int_compare(inkwell::IntPredicate::NE, int_val, zero, "int_to_bool")?
                    .into())
            }

            // Opaque type conversions (treat as i64)
            (OtterType::Opaque, OtterType::I64) | (OtterType::I64, OtterType::Opaque) => {
                Ok(value) // Already same representation
            }
            (OtterType::F64, OtterType::Opaque) => {
                let float_val = value.into_float_value();
                Ok(self
                    .builder
                    .build_bit_cast(float_val, self.context.i64_type(), "f64_to_opaque")?)
            }
            (OtterType::Opaque, OtterType::F64) => {
                let int_val = value.into_int_value();
                Ok(self
                    .builder
                    .build_bit_cast(int_val, self.context.f64_type(), "opaque_to_f64")?)
            }
            (OtterType::Bool, OtterType::Opaque) => {
                let bool_val = value.into_int_value();
                Ok(self
                    .builder
                    .build_int_z_extend(bool_val, self.context.i64_type(), "bool_to_opaque")?
                    .into())
            }
            (OtterType::Opaque, OtterType::Bool) => {
                let int_val = value.into_int_value();
                let zero = int_val.get_type().const_zero();
                Ok(self
                    .builder
                    .build_int_compare(inkwell::IntPredicate::NE, int_val, zero, "opaque_to_bool")?
                    .into())
            }

            // List/Map conversions (treat as opaque pointers)
            (OtterType::List | OtterType::Map, OtterType::Opaque)
            | (OtterType::Opaque, OtterType::List | OtterType::Map) => {
                Ok(value) // Already same representation
            }

            // Incompatible types
            _ => {
                eprintln!("DEBUG: FAILED coerce_type {:?} -> {:?}", from_ty, to_ty);
                bail!("Cannot coerce type {:?} to {:?}", from_ty, to_ty)
            }
        }
    }

    fn cast_argument_for_call(
        &self,
        value: BasicValueEnum<'ctx>,
        from_ty: OtterType,
        param_type: &BasicTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        if from_ty == OtterType::F64 && param_type.is_int_type() {
            let float_val = value.into_float_value();
            Ok(self
                .builder
                .build_float_to_signed_int(float_val, param_type.into_int_type(), "ftoi")?
                .into())
        } else if value.get_type() != *param_type
            && value.get_type().is_struct_type()
            && param_type.is_struct_type()
        {
            // Repack aggregates whose LLVM struct names don't match (e.g., named Otter structs
            // passed to anonymous FFI structs).
            let current_function = self
                .builder
                .get_insert_block()
                .and_then(|bb| bb.get_parent())
                .ok_or_else(|| anyhow!("Cannot determine current function for argument cast"))?;
            let tmp = self.create_entry_block_alloca(current_function, "struct_cast", from_ty)?;
            self.builder.build_store(tmp, value)?;
            let generic_ptr = self.context.ptr_type(AddressSpace::default());
            let cast_ptr = self
                .builder
                .build_bit_cast(tmp, generic_ptr, "struct_cast_ptr")?
                .into_pointer_value();
            let loaded = self
                .builder
                .build_load(*param_type, cast_ptr, "struct_cast_load")?;
            Ok(loaded)
        } else {
            Ok(value)
        }
    }

    fn eval_call_expr(
        &mut self,
        expr: &Expr,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        if let Expr::Call { func, args } = expr {
            let mut implicit_self: Option<EvaluatedValue<'ctx>> = None;
            if let Some(enum_value) =
                self.try_build_enum_constructor(expr, func.as_ref().as_ref(), args, ctx)?
            {
                return Ok(enum_value);
            }

            // Evaluate function expression
            let func_name = match func.as_ref().as_ref() {
                Expr::Identifier(name) => name.clone(),
                Expr::Member { object, field } => {
                    // First, try to evaluate the object to check its runtime type
                    // This handles cases like list.append() where the object is a variable
                    if let Ok(evaluated) = self.eval_expr(object.as_ref().as_ref(), ctx) {
                        if evaluated.value.is_some() {
                            // Check if it's a list type and handle list methods
                            if let OtterType::List = evaluated.ty.clone() {
                                if field == "append" && !args.is_empty() {
                                    // Determine the append function based on argument type
                                    let arg_val = self.eval_expr(args[0].as_ref(), ctx)?;
                                    let method_name: String = match arg_val.ty {
                                        OtterType::Str => "append<list,string>".to_string(),
                                        OtterType::I64 | OtterType::I32 => {
                                            "append<list,int>".to_string()
                                        }
                                        OtterType::F64 => "append<list,float>".to_string(),
                                        OtterType::Bool => "append<list,bool>".to_string(),
                                        OtterType::List | OtterType::Opaque => {
                                            "append<list,list>".to_string()
                                        }
                                        _ => bail!(
                                            "unsupported append argument type: {:?}",
                                            arg_val.ty
                                        ),
                                    };
                                    implicit_self = Some(evaluated);
                                    method_name
                                } else {
                                    bail!(
                                        "list method '{}' not supported or missing arguments",
                                        field
                                    );
                                }
                            } else if let OtterType::Struct(struct_id) = evaluated.ty.clone() {
                                if let Some(method_name) =
                                    self.resolve_struct_method_name(struct_id, field)
                                {
                                    implicit_self = Some(evaluated);
                                    method_name
                                } else {
                                    bail!(
                                        "struct method '{}.{}' not found",
                                        self.struct_info(struct_id).name,
                                        field
                                    );
                                }
                            } else {
                                // Not a list or struct, continue with other checks
                                if let Expr::Identifier(enum_name) = object.as_ref().as_ref() {
                                    if let Some(enum_value) = self
                                        .try_build_enum_from_member(enum_name, field, args, ctx)?
                                    {
                                        return Ok(enum_value);
                                    }
                                    format!("{}.{}", enum_name, field)
                                } else {
                                    bail!("Complex member expressions not yet supported");
                                }
                            }
                        } else {
                            // Value is None, try enum constructor path
                            if let Expr::Identifier(enum_name) = object.as_ref().as_ref() {
                                if let Some(enum_value) =
                                    self.try_build_enum_from_member(enum_name, field, args, ctx)?
                                {
                                    return Ok(enum_value);
                                }
                                format!("{}.{}", enum_name, field)
                            } else {
                                bail!("cannot call member '{}' without value", field);
                            }
                        }
                    } else if let Expr::Identifier(enum_name) = object.as_ref().as_ref() {
                        // Fallback: try enum constructor
                        if let Some(enum_value) =
                            self.try_build_enum_from_member(enum_name, field, args, ctx)?
                        {
                            return Ok(enum_value);
                        }
                        format!("{}.{}", enum_name, field)
                    } else if let Some(OtterType::Struct(struct_id)) =
                        self.struct_type_from_expr(object.as_ref().as_ref())
                    {
                        let self_value = self.eval_expr(object.as_ref().as_ref(), ctx)?;
                        if self_value.value.is_none() {
                            bail!("cannot call method '{}' without value", field);
                        }
                        if let Some(method_name) = self.resolve_struct_method_name(struct_id, field)
                        {
                            implicit_self = Some(self_value);
                            method_name
                        } else {
                            bail!(
                                "struct method '{}.{}' not found",
                                self.struct_info(struct_id).name,
                                field
                            );
                        }
                    } else {
                        // Evaluate the object first to check its runtime type
                        let evaluated = self.eval_expr(object.as_ref().as_ref(), ctx)?;
                        if evaluated.value.is_none() {
                            bail!("cannot call member '{}' without value", field);
                        }

                        // Check if it's a list type and handle list methods
                        if let OtterType::List = evaluated.ty.clone() {
                            // Handle list method calls like list.append()
                            if field == "append" && !args.is_empty() {
                                // Determine the append function based on argument type
                                let arg_val = self.eval_expr(args[0].as_ref(), ctx)?;
                                let method_name: String = match arg_val.ty {
                                    OtterType::Str => "append<list,string>".to_string(),
                                    OtterType::I64 | OtterType::I32 => {
                                        "append<list,int>".to_string()
                                    }
                                    OtterType::F64 => "append<list,float>".to_string(),
                                    OtterType::Bool => "append<list,bool>".to_string(),
                                    OtterType::List | OtterType::Opaque => {
                                        "append<list,list>".to_string()
                                    }
                                    _ => {
                                        bail!("unsupported append argument type: {:?}", arg_val.ty)
                                    }
                                };
                                implicit_self = Some(evaluated);
                                method_name
                            } else {
                                bail!("list method '{}' not supported or missing arguments", field);
                            }
                        } else if let OtterType::Struct(struct_id) = evaluated.ty.clone() {
                            if let Some(method_name) =
                                self.resolve_struct_method_name(struct_id, field)
                            {
                                implicit_self = Some(evaluated);
                                method_name
                            } else {
                                bail!(
                                    "struct method '{}.{}' not found",
                                    self.struct_info(struct_id).name,
                                    field
                                );
                            }
                        } else if let Some(func_name) =
                            self.resolve_member_function_name(object.as_ref().as_ref(), field)
                        {
                            func_name
                        } else {
                            bail!("Complex member expressions not yet supported (call)");
                        }
                    }
                }
                _ => bail!("Complex function expressions not yet supported"),
            };

            // Handle overloaded builtins like len() - evaluate first arg to determine type
            let (function, resolved_func_name, first_arg_evaluated) =
                if func_name == "len" && !args.is_empty() {
                    // Evaluate the first argument to determine its type
                    let arg_val = self.eval_expr(args[0].as_ref(), ctx)?;
                    let overloaded_name = match arg_val.ty {
                        OtterType::Str => "len".to_string(),
                        OtterType::List => "len<list>".to_string(),
                        OtterType::Map => "len<map>".to_string(),
                        _ => bail!("len() not supported for type {:?}", arg_val.ty),
                    };
                    if self.symbol_registry.contains(&overloaded_name) {
                        (
                            self.get_or_declare_ffi_function(&overloaded_name)?,
                            overloaded_name,
                            Some(arg_val),
                        )
                    } else {
                        bail!("Function {} not found", overloaded_name);
                    }
                } else if let Some(func) = self.declared_functions.get(&func_name) {
                    (*func, func_name.clone(), None)
                } else if self.symbol_registry.contains(&func_name) {
                    (
                        self.get_or_declare_ffi_function(&func_name)?,
                        func_name.clone(),
                        None,
                    )
                } else {
                    bail!("Function {} not found", func_name);
                };

            // Get parameter types upfront to avoid borrow issues
            let param_types: Vec<BasicTypeEnum> = function
                .get_param_iter()
                .map(|arg| arg.get_type())
                .collect();

            // Evaluate arguments and convert types as needed
            let mut arg_values: Vec<BasicMetadataValueEnum> = Vec::new();
            let mut param_offset = 0;

            if let Some(self_arg) = implicit_self {
                let v = self_arg
                    .value
                    .ok_or_else(|| anyhow!("Cannot pass unit value as self"))?;
                let param_type = param_types.first().ok_or_else(|| {
                    anyhow!("Method '{}' missing self parameter", resolved_func_name)
                })?;
                let converted = self.cast_argument_for_call(v, self_arg.ty.clone(), param_type)?;
                arg_values.push(converted.into());
                param_offset = 1;
            }

            for (i, arg) in args.iter().enumerate() {
                // Reuse first arg if it was already evaluated for len() dispatch
                let arg_val = if i == 0 {
                    if let Some(val) = first_arg_evaluated.as_ref() {
                        val.clone()
                    } else {
                        self.eval_expr(arg.as_ref(), ctx)?
                    }
                } else {
                    self.eval_expr(arg.as_ref(), ctx)?
                };
                if let Some(v) = arg_val.value {
                    let param_type = param_types.get(i + param_offset).ok_or_else(|| {
                        anyhow!("Too many arguments for function {}", resolved_func_name)
                    })?;
                    let converted =
                        self.cast_argument_for_call(v, arg_val.ty.clone(), param_type)?;
                    arg_values.push(converted.into());
                } else {
                    bail!("Cannot pass unit value as argument");
                }
            }

            // Call the function
            let call_site = self.builder.build_call(function, &arg_values, &func_name)?;

            // Get return value
            if let Some(ret_val) = call_site.try_as_basic_value().left() {
                // Use declared return type if available, otherwise infer from LLVM type
                let return_ty = self
                    .function_return_types
                    .get(&resolved_func_name)
                    .cloned()
                    .unwrap_or_else(|| {
                        function
                            .get_type()
                            .get_return_type()
                            .map(|ty| self.otter_type_from_basic_type(ty))
                            .unwrap_or(OtterType::Opaque)
                    });
                Ok(EvaluatedValue::with_value(ret_val, return_ty))
            } else {
                // Function returns void
                Ok(EvaluatedValue {
                    ty: OtterType::Unit,
                    value: None,
                })
            }
        } else {
            bail!("Expected Call expression");
        }
    }

    fn eval_if_expr(
        &mut self,
        expr: &Expr,
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        if let Expr::If {
            cond,
            then_branch,
            else_branch,
        } = expr
        {
            // Evaluate condition
            let cond_val = self.eval_expr(cond.as_ref().as_ref(), ctx)?;
            let cond_bool = self.to_bool_value(cond_val)?;

            // Get current function
            let function = self
                .builder
                .get_insert_block()
                .and_then(|bb| bb.get_parent())
                .ok_or_else(|| anyhow!("No parent function"))?;

            // Create basic blocks
            let then_bb = self.context.append_basic_block(function, "then");
            let else_bb = self.context.append_basic_block(function, "else");
            let merge_bb = self.context.append_basic_block(function, "merge");

            // Branch based on condition
            self.builder
                .build_conditional_branch(cond_bool, then_bb, else_bb)?;

            // Build then branch
            self.builder.position_at_end(then_bb);
            let then_val = self.eval_expr(then_branch.as_ref().as_ref(), ctx)?;
            let then_bb_end = self.builder.get_insert_block().unwrap();

            // Only add branch if block doesn't already terminate
            if then_bb_end.get_terminator().is_none() {
                self.builder.build_unconditional_branch(merge_bb)?;
            }

            // Build else branch
            self.builder.position_at_end(else_bb);
            let else_val = if let Some(else_br) = else_branch {
                self.eval_expr(else_br.as_ref().as_ref(), ctx)?
            } else {
                // No else branch - return unit
                EvaluatedValue {
                    ty: OtterType::Unit,
                    value: None,
                }
            };
            let else_bb_end = self.builder.get_insert_block().unwrap();

            // Only add branch if block doesn't already terminate
            if else_bb_end.get_terminator().is_none() {
                self.builder.build_unconditional_branch(merge_bb)?;
            }

            // Position at merge block
            self.builder.position_at_end(merge_bb);

            // If both branches return the same type and have values, create a phi node
            let then_ty = then_val.ty.clone();
            let else_ty = else_val.ty.clone();

            if then_ty == else_ty && then_val.value.is_some() && else_val.value.is_some() {
                if let Some(basic_ty) = self.basic_type(then_ty.clone())? {
                    let phi = self.builder.build_phi(basic_ty, "if_result")?;
                    phi.add_incoming(&[
                        (&then_val.value.unwrap(), then_bb_end),
                        (&else_val.value.unwrap(), else_bb_end),
                    ]);
                    Ok(EvaluatedValue::with_value(phi.as_basic_value(), then_ty))
                } else {
                    // Unit type
                    Ok(EvaluatedValue {
                        ty: OtterType::Unit,
                        value: None,
                    })
                }
            } else {
                // Different types or unit - return unit
                Ok(EvaluatedValue {
                    ty: OtterType::Unit,
                    value: None,
                })
            }
        } else {
            bail!("Expected If expression");
        }
    }

    fn build_string_concat(
        &mut self,
        lhs: EvaluatedValue<'ctx>,
        rhs: EvaluatedValue<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        let left_ptr = self.ensure_string_value(lhs)?;
        let right_ptr = self.ensure_string_value(rhs)?;
        let result = self.call_ffi_returning_value(
            "std.strings.concat",
            vec![left_ptr, right_ptr],
            "str_concat",
        )?;
        Ok(EvaluatedValue::with_value(result, OtterType::Str))
    }

    fn eval_array_expr(
        &mut self,
        elements: &[Node<Expr>],
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<EvaluatedValue<'ctx>> {
        // Create a new empty list
        let create_fn = self.get_or_declare_ffi_function("list.new")?;
        let handle = self
            .builder
            .build_call(create_fn, &[], "list_handle")?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("list creation returned void"))?
            .into_int_value();

        // Append each element to the list
        for (idx, elem) in elements.iter().enumerate() {
            let elem_val = self.eval_expr(elem.as_ref(), ctx)?;
            let elem_value = elem_val
                .value
                .ok_or_else(|| anyhow!("array element {} produced no value", idx))?;

            // Determine which append function to use based on element type
            let append_fn_name = match elem_val.ty {
                OtterType::Str => "append<list,string>",
                OtterType::I64 | OtterType::I32 => "append<list,int>",
                OtterType::F64 => "append<list,float>",
                OtterType::Bool => "append<list,bool>",
                OtterType::List | OtterType::Opaque => "append<list,list>",
                _ => bail!("unsupported array element type: {:?}", elem_val.ty),
            };

            let append_fn = self.get_or_declare_ffi_function(append_fn_name)?;
            let handle_arg: BasicMetadataValueEnum = handle.into();
            let elem_arg: BasicMetadataValueEnum = elem_value.into();

            self.builder.build_call(
                append_fn,
                &[handle_arg, elem_arg],
                &format!("append_{}", idx),
            )?;
        }

        Ok(EvaluatedValue::with_value(handle.into(), OtterType::List))
    }

    fn ensure_string_value(&mut self, value: EvaluatedValue<'ctx>) -> Result<BasicValueEnum<'ctx>> {
        let EvaluatedValue { ty, value } = value;
        let base_value = value.ok_or_else(|| anyhow!("expected value for string operation"))?;

        match ty {
            OtterType::Str => Ok(base_value),
            OtterType::I64 => {
                self.call_ffi_returning_value("std.strings.format_int", vec![base_value], "fmt_int")
            }
            OtterType::I32 => {
                let int_val = base_value.into_int_value();
                let widened = self.builder.build_int_s_extend(
                    int_val,
                    self.context.i64_type(),
                    "i32_to_i64",
                )?;
                self.call_ffi_returning_value(
                    "std.strings.format_int",
                    vec![widened.into()],
                    "fmt_int",
                )
            }
            OtterType::F64 => self.call_ffi_returning_value(
                "std.strings.format_float",
                vec![base_value],
                "fmt_float",
            ),
            OtterType::Bool => self.call_ffi_returning_value(
                "std.strings.format_bool",
                vec![base_value],
                "fmt_bool",
            ),
            OtterType::List | OtterType::Opaque => {
                // Try to convert list handle to string
                // Opaque types might be list handles, so try stringify
                self.call_ffi_returning_value("stringify<list>", vec![base_value], "stringify_list")
            }
            _ => bail!("cannot convert {:?} to string", ty),
        }
    }

    fn call_ffi_returning_value(
        &mut self,
        name: &str,
        args: Vec<BasicValueEnum<'ctx>>,
        label: &str,
    ) -> Result<BasicValueEnum<'ctx>> {
        let function = self.get_or_declare_ffi_function(name)?;
        let metadata_args: Vec<BasicMetadataValueEnum> =
            args.into_iter().map(|arg| arg.into()).collect();
        let call = self.builder.build_call(function, &metadata_args, label)?;
        call.try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("FFI function {name} returned void"))
    }

    fn try_build_enum_constructor(
        &mut self,
        call_expr: &Expr,
        func_expr: &Expr,
        args: &[Node<Expr>],
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<Option<EvaluatedValue<'ctx>>> {
        if let (Expr::Member { field, .. }, Some(enum_type @ TypeInfo::Enum { .. })) =
            (func_expr, self.expr_type(call_expr).cloned())
        {
            let mut evaluated_args = Vec::with_capacity(args.len());
            for arg in args {
                evaluated_args.push(self.eval_expr(arg.as_ref(), ctx)?);
            }
            let value = self.build_enum_value_from_type(&enum_type, field, evaluated_args)?;
            return Ok(Some(value));
        }
        Ok(None)
    }

    fn try_build_enum_from_member(
        &mut self,
        enum_name: &str,
        variant_name: &str,
        args: &[Node<Expr>],
        ctx: &mut FunctionContext<'ctx>,
    ) -> Result<Option<EvaluatedValue<'ctx>>> {
        if let Some(layout) = self.enum_layout(enum_name) {
            let tag = layout.tag_of(variant_name);
            if tag.is_none() {
                return Ok(None);
            }
            let tag = tag.unwrap();

            let mut evaluated_args = Vec::with_capacity(args.len());
            for arg in args {
                evaluated_args.push(self.eval_expr(arg.as_ref(), ctx)?);
            }

            let field_types: Vec<TypeInfo> = evaluated_args
                .iter()
                .map(|val| match val.ty.clone() {
                    OtterType::I64 => TypeInfo::I64,
                    OtterType::F64 => TypeInfo::F64,
                    OtterType::Bool => TypeInfo::Bool,
                    OtterType::Str => TypeInfo::Str,
                    _ => TypeInfo::Unknown,
                })
                .collect();

            let value = self.create_enum_instance(
                enum_name,
                variant_name,
                tag,
                &field_types,
                evaluated_args,
            )?;

            return Ok(Some(value));
        }
        Ok(None)
    }

    #[allow(clippy::collapsible_if)]
    fn try_build_enum_member(
        &mut self,
        expr: &Expr,
        _object: &Expr,
        field: &str,
        _ctx: &mut FunctionContext<'ctx>,
    ) -> Result<Option<EvaluatedValue<'ctx>>> {
        if let Some(enum_type_ref @ TypeInfo::Enum { variants, .. }) = self.expr_type(expr) {
            if let Some(variant) = variants.get(field)
                && variant.fields.is_empty()
            {
                let enum_type = enum_type_ref.clone();
                return Ok(Some(self.build_enum_value_from_type(
                    &enum_type,
                    field,
                    Vec::new(),
                )?));
            }
        }
        Ok(None)
    }

    fn module_path_from_expr(&self, expr: &Expr) -> Option<String> {
        self.expr_type(expr).and_then(|ty| match ty {
            TypeInfo::Module(name) => Some(name.clone()),
            _ => None,
        })
    }

    fn resolve_member_function_name(&self, object: &Expr, field: &str) -> Option<String> {
        if let Some(module) = self.module_path_from_expr(object) {
            let candidate = format!("{}.{}", module, field);
            if self.symbol_registry.contains(&candidate)
                || self.declared_functions.contains_key(&candidate)
            {
                return Some(candidate);
            }
        }

        let prefix = self.flatten_member_chain(object)?;
        let candidate = format!("{}.{}", prefix, field);
        if self.symbol_registry.contains(&candidate) {
            Some(candidate)
        } else {
            None
        }
    }

    fn flatten_member_chain(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Identifier(name) => Some(name.clone()),
            Expr::Member { object, field } => {
                let prefix = self.flatten_member_chain(object.as_ref().as_ref())?;
                Some(format!("{}.{}", prefix, field))
            }
            _ => None,
        }
    }

    fn build_enum_value_from_type(
        &mut self,
        enum_type: &TypeInfo,
        variant_name: &str,
        values: Vec<EvaluatedValue<'ctx>>,
    ) -> Result<EvaluatedValue<'ctx>> {
        if let TypeInfo::Enum { name, variants, .. } = enum_type {
            let layout = self
                .enum_layout(name)
                .ok_or_else(|| anyhow!("Missing enum layout for {name}"))?;
            let tag = layout
                .tag_of(variant_name)
                .ok_or_else(|| anyhow!("Unknown variant {name}.{variant_name}"))?;
            let variant_info = variants
                .get(variant_name)
                .ok_or_else(|| anyhow!("No variant named {variant_name} for enum {name}"))?;
            if variant_info.fields.len() != values.len() {
                bail!(
                    "enum variant {}.{} expects {} field(s), got {}",
                    name,
                    variant_name,
                    variant_info.fields.len(),
                    values.len()
                );
            }

            self.create_enum_instance(name, variant_name, tag, &variant_info.fields, values)
        } else {
            bail!("expected enum type when constructing variant {variant_name}");
        }
    }

    fn create_enum_instance(
        &mut self,
        _enum_name: &str,
        _variant_name: &str,
        tag: u32,
        field_types: &[TypeInfo],
        values: Vec<EvaluatedValue<'ctx>>,
    ) -> Result<EvaluatedValue<'ctx>> {
        let i64_type = self.context.i64_type();
        let tag_value = i64_type.const_int(tag as u64, false);
        let field_count = i64_type.const_int(field_types.len() as u64, false);
        let create_fn = self.get_or_declare_ffi_function("runtime.enum.create")?;
        let handle = self
            .builder
            .build_call(
                create_fn,
                &[tag_value.into(), field_count.into()],
                "enum_create",
            )?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow!("runtime.enum.create returned void"))?;

        for (index, (field_type, value)) in field_types.iter().zip(values.into_iter()).enumerate() {
            self.store_enum_field(handle, index as u32, field_type, value)?;
        }

        Ok(EvaluatedValue::with_value(handle, OtterType::Opaque))
    }

    fn store_enum_field(
        &mut self,
        handle: BasicValueEnum<'ctx>,
        index: u32,
        field_type: &TypeInfo,
        value: EvaluatedValue<'ctx>,
    ) -> Result<()> {
        let i64_type = self.context.i64_type();
        let index_val = i64_type.const_int(index as u64, false);
        let handle_arg: BasicMetadataValueEnum = handle.into();
        let index_arg: BasicMetadataValueEnum = index_val.into();

        match enum_field_kind(field_type) {
            EnumFieldKind::Int => {
                let data = self.value_as_i64(value)?;
                let setter = self.get_or_declare_ffi_function("runtime.enum.set_i64")?;
                self.builder.build_call(
                    setter,
                    &[handle_arg, index_arg, data.into()],
                    "enum_set_i64",
                )?;
            }
            EnumFieldKind::Float => {
                let data = self.value_as_f64(value)?;
                let setter = self.get_or_declare_ffi_function("runtime.enum.set_f64")?;
                self.builder.build_call(
                    setter,
                    &[handle_arg, index_arg, data.into()],
                    "enum_set_f64",
                )?;
            }
            EnumFieldKind::Bool => {
                let data = self.value_as_bool(value)?;
                let setter = self.get_or_declare_ffi_function("runtime.enum.set_bool")?;
                self.builder.build_call(
                    setter,
                    &[handle_arg, index_arg, data.into()],
                    "enum_set_bool",
                )?;
            }
            EnumFieldKind::Ptr => {
                let data = self.value_as_i64(value)?;
                let setter = self.get_or_declare_ffi_function("runtime.enum.set_ptr")?;
                self.builder.build_call(
                    setter,
                    &[handle_arg, index_arg, data.into()],
                    "enum_set_ptr",
                )?;
            }
        }

        Ok(())
    }

    fn value_as_i64(&mut self, value: EvaluatedValue<'ctx>) -> Result<IntValue<'ctx>> {
        let EvaluatedValue { ty, value } = value;
        let raw = value.ok_or_else(|| anyhow!("missing value for enum field"))?;
        let int_value = match ty {
            OtterType::I64 | OtterType::Opaque => raw.into_int_value(),
            OtterType::I32 => self.builder.build_int_s_extend(
                raw.into_int_value(),
                self.context.i64_type(),
                "i32_to_i64",
            )?,
            OtterType::Bool => self.builder.build_int_z_extend(
                raw.into_int_value(),
                self.context.i64_type(),
                "bool_to_i64",
            )?,
            OtterType::Str => self.builder.build_ptr_to_int(
                raw.into_pointer_value(),
                self.context.i64_type(),
                "str_ptr_to_int",
            )?,
            _ => {
                bail!("cannot convert {:?} to i64 for enum field", ty);
            }
        };
        Ok(int_value)
    }

    fn value_as_f64(&mut self, value: EvaluatedValue<'ctx>) -> Result<BasicValueEnum<'ctx>> {
        let EvaluatedValue { ty, value } = value;
        let raw = value.ok_or_else(|| anyhow!("missing value for enum field"))?;
        let float_value = match ty {
            OtterType::F64 => raw.into_float_value(),
            OtterType::I64 => self.builder.build_signed_int_to_float(
                raw.into_int_value(),
                self.context.f64_type(),
                "i64_to_f64",
            )?,
            OtterType::I32 => self.builder.build_signed_int_to_float(
                raw.into_int_value(),
                self.context.f64_type(),
                "i32_to_f64",
            )?,
            _ => {
                bail!("cannot convert {:?} to f64 for enum field", ty);
            }
        };
        Ok(float_value.into())
    }

    fn value_as_bool(&self, value: EvaluatedValue<'ctx>) -> Result<IntValue<'ctx>> {
        let EvaluatedValue { ty, value } = value;
        if ty == OtterType::Bool {
            Ok(value
                .ok_or_else(|| anyhow!("missing bool value for enum field"))?
                .into_int_value())
        } else {
            bail!("expected bool value for enum field, got {:?}", ty);
        }
    }
}

enum EnumFieldKind {
    Int,
    Float,
    Bool,
    Ptr,
}

fn enum_field_kind(field_type: &TypeInfo) -> EnumFieldKind {
    match field_type {
        TypeInfo::Bool => EnumFieldKind::Bool,
        TypeInfo::I32 | TypeInfo::I64 => EnumFieldKind::Int,
        TypeInfo::F64 => EnumFieldKind::Float,
        TypeInfo::Alias { underlying, .. } => enum_field_kind(underlying),
        _ => EnumFieldKind::Ptr,
    }
}
