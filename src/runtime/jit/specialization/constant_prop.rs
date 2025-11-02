use super::RuntimeConstant;
use crate::ast::nodes::Expr;

/// Propagates constant values through expressions
pub struct ConstantPropagator;

impl ConstantPropagator {
    pub fn new() -> Self {
        Self
    }

    /// Analyze expression to extract constant values
    pub fn extract_constants(&self, expr: &Expr) -> Vec<Option<RuntimeConstant>> {
        match expr {
            Expr::Literal(lit) => {
                vec![Some(self.literal_to_constant(lit))]
            }
            Expr::Binary { left, right, .. } => {
                let mut result = self.extract_constants(left);
                result.extend(self.extract_constants(right));
                result
            }
            Expr::Call { args, .. } => args
                .iter()
                .flat_map(|arg| self.extract_constants(arg))
                .collect(),
            _ => vec![None],
        }
    }

    fn literal_to_constant(&self, lit: &crate::ast::nodes::Literal) -> RuntimeConstant {
        match lit {
            crate::ast::nodes::Literal::Bool(b) => RuntimeConstant::Bool(*b),
            crate::ast::nodes::Literal::Number(n) => {
                // Try to determine if it's an integer or float
                if n.fract() == 0.0 {
                    if *n >= i32::MIN as f64 && *n <= i32::MAX as f64 {
                        RuntimeConstant::I32(*n as i32)
                    } else {
                        RuntimeConstant::I64(*n as i64)
                    }
                } else {
                    RuntimeConstant::F64(*n)
                }
            }
            crate::ast::nodes::Literal::String(s) => RuntimeConstant::Str(s.clone()),
        }
    }
}

impl Default for ConstantPropagator {
    fn default() -> Self {
        Self::new()
    }
}
