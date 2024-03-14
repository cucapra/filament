use super::{Binding, Expr, Time, TimeSub};

/// Ordering operator for constraints
#[derive(Hash, Eq, PartialEq, Clone)]
pub enum OrderOp {
    Gt,
    Gte,
    Eq,
}
impl std::fmt::Display for OrderOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self {
            OrderOp::Gt => ">",
            OrderOp::Eq => "=",
            OrderOp::Gte => ">=",
        };
        write!(f, "{op}")
    }
}

// An ordering constraint
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct OrderConstraint<T> {
    pub left: T,
    pub right: T,
    pub op: OrderOp,
}

impl<T> OrderConstraint<T>
where
    T: Clone,
{
    pub fn new(left: T, right: T, op: OrderOp) -> Self {
        Self { left, right, op }
    }

    pub fn is_eq(&self) -> bool {
        self.op == OrderOp::Eq
    }

    pub fn gt(l: T, r: T) -> Self {
        Self {
            left: l,
            right: r,
            op: OrderOp::Gt,
        }
    }

    pub fn lt(l: T, r: T) -> Self {
        Self {
            left: r,
            right: l,
            op: OrderOp::Gt,
        }
    }

    pub fn eq(left: T, right: T) -> Self {
        OrderConstraint {
            left,
            right,
            op: OrderOp::Eq,
        }
    }

    pub fn gte(left: T, right: T) -> Self {
        OrderConstraint {
            left,
            right,
            op: OrderOp::Gte,
        }
    }

    pub fn lte(l: T, r: T) -> Self {
        OrderConstraint {
            left: r,
            right: l,
            op: OrderOp::Gte,
        }
    }
}

impl OrderConstraint<Expr> {
    pub fn resolve_expr(self, binding: &Binding<Expr>) -> Self {
        OrderConstraint {
            left: self.left.resolve(binding),
            right: self.right.resolve(binding),
            ..self
        }
    }

    pub fn exprs(&self) -> Vec<&Expr> {
        vec![&self.left, &self.right]
    }
}

impl OrderConstraint<Time> {
    pub fn resolve_event(self, bindings: &Binding<Time>) -> Self {
        OrderConstraint {
            left: self.left.resolve_event(bindings),
            right: self.right.resolve_event(bindings),
            ..self
        }
    }

    pub fn resolve_expr(self, bindings: &Binding<Expr>) -> Self {
        OrderConstraint {
            left: self.left.resolve_expr(bindings),
            right: self.right.resolve_expr(bindings),
            ..self
        }
    }
}

impl OrderConstraint<TimeSub> {
    fn resolve_event(self, bindings: &Binding<Time>) -> Self {
        OrderConstraint {
            left: self.left.resolve_event(bindings),
            right: self.right.resolve_event(bindings),
            ..self
        }
    }

    fn resolve_expr(self, bindings: &Binding<Expr>) -> Self {
        OrderConstraint {
            left: self.left.resolve_expr(bindings),
            right: self.right.resolve_expr(bindings),
            ..self
        }
    }
}

/// A ordering constraint over time expressions or time ranges.
#[derive(Clone, Hash)]
pub enum Constraint {
    Base {
        base: OrderConstraint<Time>,
    },
    /// Represents ordering over time ranges.
    Sub {
        base: OrderConstraint<TimeSub>,
    },
}

impl Constraint {
    pub fn base(base: OrderConstraint<Time>) -> Self {
        Self::Base { base }
    }

    pub fn sub(base: OrderConstraint<TimeSub>) -> Self {
        Self::Sub { base }
    }

    /// Create a new constraint that `l` is less than `r`
    pub fn lt(l: Time, r: Time) -> Self {
        Self::Base {
            base: OrderConstraint::lt(l, r),
        }
    }

    pub fn resolve_event(self, binding: &Binding<Time>) -> Constraint {
        match self {
            Constraint::Base { base } => Constraint::Base {
                base: base.resolve_event(binding),
            },
            Constraint::Sub { base } => Constraint::Sub {
                base: base.resolve_event(binding),
            },
        }
    }

    pub fn resolve_expr(self, bindings: &Binding<Expr>) -> Self {
        match self {
            Constraint::Base { base } => Constraint::Base {
                base: base.resolve_expr(bindings),
            },
            Constraint::Sub { base } => Constraint::Sub {
                base: base.resolve_expr(bindings),
            },
        }
    }
}

impl From<OrderConstraint<Time>> for Constraint {
    fn from(con: OrderConstraint<Time>) -> Self {
        Constraint::Base { base: con }
    }
}

impl From<OrderConstraint<TimeSub>> for Constraint {
    fn from(con: OrderConstraint<TimeSub>) -> Self {
        Constraint::Sub { base: con }
    }
}
