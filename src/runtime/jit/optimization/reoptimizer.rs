use std::collections::HashSet;

use super::call_graph::CallGraph;
use super::inliner::{InlineConfig, Inliner};
use crate::codegen::CodegenOptLevel;
use ast::nodes::{
    BinaryOp, Block, Expr, FStringPart, Function, Literal, NumberLiteral, Program, Statement,
    UnaryOp,
};

/// Re-optimizes hot functions
pub struct Reoptimizer {
    #[allow(dead_code)]
    opt_level: CodegenOptLevel,
    hot_functions: HashSet<String>,
    inliner: Inliner,
}

impl Reoptimizer {
    pub fn new() -> Self {
        Self::with_opt_level(CodegenOptLevel::Aggressive)
    }

    pub fn with_opt_level(opt_level: CodegenOptLevel) -> Self {
        let inline_config = match opt_level {
            CodegenOptLevel::None => InlineConfig {
                max_inline_size: 24,
                max_depth: 1,
                inline_hot_only: true,
            },
            CodegenOptLevel::Default => InlineConfig {
                max_inline_size: 48,
                max_depth: 2,
                inline_hot_only: true,
            },
            CodegenOptLevel::Aggressive => InlineConfig {
                max_inline_size: 80,
                max_depth: 3,
                inline_hot_only: false,
            },
        };

        Self {
            opt_level,
            hot_functions: HashSet::new(),
            inliner: Inliner::with_config(inline_config),
        }
    }

    /// Provide an explicit set of hot functions discovered by the profiler.
    pub fn set_hot_functions<I, S>(&mut self, hot: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.hot_functions = hot.into_iter().map(|s| s.into()).collect();
    }

    pub fn inliner(&self) -> &Inliner {
        &self.inliner
    }

    /// Re-optimize a function by applying aggressive but semantics-preserving cleanups.
    pub fn reoptimize_function(&self, function: &Function) -> Function {
        let mut optimized = function.clone();
        self.clean_block(&mut optimized.body);
        optimized
    }

    /// Optimize hot call paths by inlining and running post-inline cleanups.
    pub fn optimize_hot_paths(&self, program: &Program, call_graph: &CallGraph) -> Program {
        let hot_candidates = if self.hot_functions.is_empty() {
            call_graph.hot_candidates(8)
        } else {
            self.hot_functions.iter().cloned().collect()
        };
        let hot_set: HashSet<String> = hot_candidates.into_iter().collect();

        let (mut optimized, _) = self.inliner.inline_program(program, &hot_set, call_graph);

        for stmt in &mut optimized.statements {
            if let Statement::Function(func) = stmt {
                if hot_set.contains(&func.name) {
                    *func = self.post_inline_optimize(func);
                } else {
                    *func = self.reoptimize_function(func);
                }
            }
        }

        optimized
    }

    /// Apply post-inline optimizations such as dead-code elimination and block flattening.
    pub fn post_inline_optimize(&self, function: &Function) -> Function {
        let mut optimized = function.clone();
        self.clean_block(&mut optimized.body);
        self.prune_empty_blocks(&mut optimized.body);
        optimized
    }

    fn clean_block(&self, block: &mut Block) {
        self.fold_constants_in_block(block);
        self.remove_dead_statements(block);
    }

    fn fold_constants_in_block(&self, block: &mut Block) {
        let mut rewritten = Vec::with_capacity(block.statements.len());
        for mut stmt in block.statements.drain(..) {
            self.fold_constants_in_statement(&mut stmt);
            match self.simplify_statement(stmt) {
                StatementTransform::Single(stmt) => rewritten.push(stmt),
                StatementTransform::Many(stmts) => rewritten.extend(stmts),
                StatementTransform::None => {}
            }
        }
        block.statements = rewritten;
    }

    fn fold_constants_in_statement(&self, stmt: &mut Statement) {
        match stmt {
            Statement::Let { expr, .. }
            | Statement::Assignment { expr, .. }
            | Statement::Expr(expr)
            | Statement::Return(Some(expr))
            | Statement::Raise(Some(expr)) => {
                self.fold_constants_in_expr(expr);
            }
            Statement::If {
                cond,
                then_block,
                elif_blocks,
                else_block,
            } => {
                self.fold_constants_in_expr(cond);
                self.fold_constants_in_block(then_block);
                for (_, block) in elif_blocks {
                    self.fold_constants_in_block(block);
                }
                if let Some(block) = else_block {
                    self.fold_constants_in_block(block);
                }
            }
            Statement::While { cond, body } => {
                self.fold_constants_in_expr(cond);
                self.fold_constants_in_block(body);
            }
            Statement::For { iterable, body, .. } => {
                self.fold_constants_in_expr(iterable);
                self.fold_constants_in_block(body);
            }
            Statement::Block(inner) => self.fold_constants_in_block(inner),
            Statement::Try {
                body,
                handlers,
                else_block,
                finally_block,
            } => {
                self.fold_constants_in_block(body);
                for handler in handlers {
                    self.fold_constants_in_block(&mut handler.body);
                }
                if let Some(block) = else_block {
                    self.fold_constants_in_block(block);
                }
                if let Some(block) = finally_block {
                    self.fold_constants_in_block(block);
                }
            }
            _ => {}
        }
    }

    fn fold_constants_in_expr(&self, expr: &mut Expr) -> Option<Literal> {
        match expr {
            Expr::Literal(lit) => Some(lit.clone()),
            Expr::Unary { op, expr: inner } => {
                let literal = self.fold_constants_in_expr(inner);
                if let Some(lit) = literal
                    && let Some(new_lit) = Self::eval_unary(*op, &lit)
                {
                    *expr = Expr::Literal(new_lit.clone());
                    return Some(new_lit);
                }
                None
            }
            Expr::Binary { op, left, right } => {
                let left_lit = self.fold_constants_in_expr(left);
                let right_lit = self.fold_constants_in_expr(right);
                if let (Some(l), Some(r)) = (left_lit, right_lit)
                    && let Some(new_lit) = Self::eval_binary(*op, &l, &r)
                {
                    *expr = Expr::Literal(new_lit.clone());
                    return Some(new_lit);
                }
                None
            }
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_lit = self.fold_constants_in_expr(cond);
                self.fold_constants_in_expr(then_branch);
                if let Some(branch) = else_branch.as_mut() {
                    self.fold_constants_in_expr(branch);
                }
                if let Some(Literal::Bool(value)) = cond_lit {
                    let replacement = if value {
                        *then_branch.clone()
                    } else if let Some(branch) = else_branch {
                        *branch.clone()
                    } else {
                        Expr::Literal(Literal::Unit)
                    };
                    *expr = replacement;
                }
                None
            }
            Expr::Call { func, args } => {
                self.fold_constants_in_expr(func);
                for arg in args {
                    self.fold_constants_in_expr(arg);
                }
                None
            }
            Expr::Array(values) => {
                for value in values {
                    self.fold_constants_in_expr(value);
                }
                None
            }
            Expr::Dict(pairs) => {
                for (key, value) in pairs {
                    self.fold_constants_in_expr(key);
                    self.fold_constants_in_expr(value);
                }
                None
            }
            Expr::ListComprehension {
                element,
                iterable,
                condition,
                ..
            } => {
                self.fold_constants_in_expr(element);
                self.fold_constants_in_expr(iterable);
                if let Some(cond) = condition {
                    self.fold_constants_in_expr(cond);
                }
                None
            }
            Expr::DictComprehension {
                key,
                value,
                iterable,
                condition,
                ..
            } => {
                self.fold_constants_in_expr(key);
                self.fold_constants_in_expr(value);
                self.fold_constants_in_expr(iterable);
                if let Some(cond) = condition {
                    self.fold_constants_in_expr(cond);
                }
                None
            }
            Expr::Match { value, arms } => {
                self.fold_constants_in_expr(value);
                for arm in arms {
                    if let Some(guard) = &mut arm.guard {
                        self.fold_constants_in_expr(guard);
                    }
                    self.fold_constants_in_expr(&mut arm.body);
                }
                None
            }
            Expr::FString { parts } => {
                for part in parts {
                    if let FStringPart::Expr(expr) = part {
                        self.fold_constants_in_expr(expr);
                    }
                }
                None
            }
            Expr::Lambda { body, .. } => {
                self.fold_constants_in_block(body);
                None
            }
            Expr::Spawn(expr) | Expr::Await(expr) => {
                self.fold_constants_in_expr(expr);
                None
            }
            Expr::Struct { fields, .. } => {
                for (_, value) in fields {
                    self.fold_constants_in_expr(value);
                }
                None
            }
            _ => None,
        }
    }

    fn eval_unary(op: UnaryOp, literal: &Literal) -> Option<Literal> {
        match (op, literal) {
            (UnaryOp::Not, Literal::Bool(value)) => Some(Literal::Bool(!value)),
            (UnaryOp::Neg, Literal::Number(num)) => Some(Literal::Number(NumberLiteral::new(
                -num.value,
                num.is_float_literal,
            ))),
            _ => None,
        }
    }

    fn eval_binary(op: BinaryOp, left: &Literal, right: &Literal) -> Option<Literal> {
        match op {
            BinaryOp::Add => Self::eval_arithmetic(left, right, |a, b| a + b),
            BinaryOp::Sub => Self::eval_arithmetic(left, right, |a, b| a - b),
            BinaryOp::Mul => Self::eval_arithmetic(left, right, |a, b| a * b),
            BinaryOp::Div => {
                if matches!(right, Literal::Number(n) if n.value == 0.0) {
                    None
                } else {
                    Self::eval_arithmetic(left, right, |a, b| a / b)
                }
            }
            BinaryOp::Mod => Self::eval_arithmetic(left, right, |a, b| a % b),
            BinaryOp::And => match (left, right) {
                (Literal::Bool(a), Literal::Bool(b)) => Some(Literal::Bool(*a && *b)),
                _ => None,
            },
            BinaryOp::Or => match (left, right) {
                (Literal::Bool(a), Literal::Bool(b)) => Some(Literal::Bool(*a || *b)),
                _ => None,
            },
            BinaryOp::Eq => Some(Literal::Bool(left == right)),
            BinaryOp::Ne => Some(Literal::Bool(left != right)),
            BinaryOp::Lt | BinaryOp::Gt | BinaryOp::LtEq | BinaryOp::GtEq => {
                if let (Literal::Number(a), Literal::Number(b)) = (left, right) {
                    let result = match op {
                        BinaryOp::Lt => a.value < b.value,
                        BinaryOp::Gt => a.value > b.value,
                        BinaryOp::LtEq => a.value <= b.value,
                        BinaryOp::GtEq => a.value >= b.value,
                        _ => unreachable!(),
                    };
                    Some(Literal::Bool(result))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn eval_arithmetic<F>(left: &Literal, right: &Literal, op: F) -> Option<Literal>
    where
        F: Fn(f64, f64) -> f64,
    {
        if let (Literal::Number(a), Literal::Number(b)) = (left, right) {
            let value = op(a.value, b.value);
            Some(Literal::Number(NumberLiteral::new(
                value,
                a.is_float_literal || b.is_float_literal,
            )))
        } else {
            None
        }
    }

    fn simplify_statement(&self, stmt: Statement) -> StatementTransform {
        match stmt {
            Statement::Pass => StatementTransform::None,
            Statement::If {
                cond,
                then_block,
                elif_blocks,
                else_block,
            } => {
                if let Expr::Literal(Literal::Bool(value)) = *cond {
                    if value {
                        StatementTransform::Many(then_block.statements)
                    } else if let Some(block) = else_block {
                        StatementTransform::Many(block.statements)
                    } else {
                        StatementTransform::None
                    }
                } else if elif_blocks.is_empty()
                    && else_block
                        .as_ref()
                        .map(|block| block.statements.is_empty())
                        .unwrap_or(true)
                    && then_block.statements.is_empty()
                {
                    StatementTransform::None
                } else {
                    StatementTransform::Single(Statement::If {
                        cond,
                        then_block,
                        elif_blocks,
                        else_block,
                    })
                }
            }
            Statement::Block(block) if block.statements.is_empty() => StatementTransform::None,
            other => StatementTransform::Single(other),
        }
    }

    fn remove_dead_statements(&self, block: &mut Block) {
        let mut pruned = Vec::with_capacity(block.statements.len());
        let mut terminated = false;

        for stmt in block.statements.drain(..) {
            if terminated {
                break;
            }
            terminated = matches!(
                stmt,
                Statement::Return(_) | Statement::Break | Statement::Continue
            );
            pruned.push(stmt);
        }

        block.statements = pruned;

        for stmt in &mut block.statements {
            match stmt {
                Statement::If {
                    then_block,
                    elif_blocks,
                    else_block,
                    ..
                } => {
                    self.remove_dead_statements(then_block);
                    for (_, block) in elif_blocks {
                        self.remove_dead_statements(block);
                    }
                    if let Some(block) = else_block {
                        self.remove_dead_statements(block);
                    }
                }
                Statement::While { body, .. }
                | Statement::For { body, .. }
                | Statement::Block(body) => self.remove_dead_statements(body),
                Statement::Try {
                    body,
                    handlers,
                    else_block,
                    finally_block,
                } => {
                    self.remove_dead_statements(body);
                    for handler in handlers {
                        self.remove_dead_statements(&mut handler.body);
                    }
                    if let Some(block) = else_block {
                        self.remove_dead_statements(block);
                    }
                    if let Some(block) = finally_block {
                        self.remove_dead_statements(block);
                    }
                }
                _ => {}
            }
        }
    }

    fn prune_empty_blocks(&self, block: &mut Block) {
        let mut flattened = Vec::with_capacity(block.statements.len());
        for mut stmt in block.statements.drain(..) {
            match &mut stmt {
                Statement::Block(inner) => {
                    self.prune_empty_blocks(inner);
                    if inner.statements.is_empty() {
                        continue;
                    }
                    flattened.push(Statement::Block(inner.clone()));
                }
                Statement::If {
                    then_block,
                    elif_blocks,
                    else_block,
                    ..
                } => {
                    self.prune_empty_blocks(then_block);
                    for (_, block) in elif_blocks {
                        self.prune_empty_blocks(block);
                    }
                    if let Some(block) = else_block {
                        self.prune_empty_blocks(block);
                    }
                    flattened.push(stmt);
                }
                Statement::While { body, .. } | Statement::For { body, .. } => {
                    self.prune_empty_blocks(body);
                    flattened.push(stmt);
                }
                Statement::Try {
                    body,
                    handlers,
                    else_block,
                    finally_block,
                } => {
                    self.prune_empty_blocks(body);
                    for handler in handlers {
                        self.prune_empty_blocks(&mut handler.body);
                    }
                    if let Some(block) = else_block {
                        self.prune_empty_blocks(block);
                    }
                    if let Some(block) = finally_block {
                        self.prune_empty_blocks(block);
                    }
                    flattened.push(stmt);
                }
                _ => flattened.push(stmt),
            }
        }
        block.statements = flattened;
    }
}

impl Default for Reoptimizer {
    fn default() -> Self {
        Self::new()
    }
}

enum StatementTransform {
    Single(Statement),
    Many(Vec<Statement>),
    None,
}
