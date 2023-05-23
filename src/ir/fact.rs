use super::{idxs::PropIdx, Component, Ctx, ExprIdx, Foldable, ParamIdx};

#[derive(Clone, PartialEq, Eq, Hash)]
/// Comparison operators
pub enum Cmp {
    Gt,
    Gte,
    Eq,
}

#[derive(Clone, PartialEq, Eq, Hash)]
/// A proposition
pub enum Prop {
    True,
    /// A comparison between two expressions
    Cmp {
        op: Cmp,
        lhs: ExprIdx,
        rhs: ExprIdx,
    },
    Not(PropIdx),
    And(PropIdx, PropIdx),
    Or(PropIdx, PropIdx),
    Implies(PropIdx, PropIdx),
}

impl PropIdx {
    #[inline]
    /// Returns true of this proposition is definitely true.
    /// This is a purely syntactic check, and does not attempt to reduce the
    /// proposition.
    pub fn is_true(&self, ctx: &impl Ctx<Prop>) -> bool {
        matches!(ctx.get(*self), Prop::True)
    }

    #[inline]
    /// Returns true of this proposition is definitely false.
    /// This is a purely syntactic check, and does not attempt to reduce the
    /// proposition.
    pub fn is_false(&self, ctx: &impl Ctx<Prop>) -> bool {
        matches!(
            ctx.get(*self), Prop::Not(p)
            if matches!(ctx.get(*p), Prop::True)
        )
    }

    /// Negation of a proposition
    pub fn not(self, ctx: &mut impl Ctx<Prop>) -> PropIdx {
        if self.is_false(ctx) {
            return ctx.add(Prop::True);
        }

        ctx.add(Prop::Not(self))
    }

    /// Conjunction of two propositions
    pub fn and(self, other: PropIdx, ctx: &mut impl Ctx<Prop>) -> PropIdx {
        if self == other {
            return self;
        } else if self.is_true(ctx) {
            return other;
        } else if other.is_true(ctx) {
            return self;
        }

        // Canonicalize And by sorting the operands
        let (l, r) = if self < other {
            (self, other)
        } else {
            (other, self)
        };
        ctx.add(Prop::And(l, r))
    }

    /// Disjunction of two propositions
    pub fn or(self, other: PropIdx, ctx: &mut impl Ctx<Prop>) -> PropIdx {
        if self == other {
            return self;
        } else if self.is_false(ctx) {
            return other;
        } else if other.is_false(ctx) {
            return self;
        }

        // Canonicalize Or by sorting the operands
        let (l, r) = if self < other {
            (self, other)
        } else {
            (other, self)
        };
        ctx.add(Prop::Or(l, r))
    }

    pub fn implies(self, cons: PropIdx, ctx: &mut impl Ctx<Prop>) -> PropIdx {
        // If the proposition is false, then the implication is trivially true
        if self.is_false(ctx) {
            // Warning because its not clear if this is ever expected behavior
            log::warn!("A false proposition was created");
            return ctx.add(Prop::True);
        } else if cons.is_true(ctx) {
            return ctx.add(Prop::True);
        } else if self.is_true(ctx) {
            return cons;
        }

        ctx.add(Prop::Implies(self, cons))
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
/// A fact in the program.
/// If `checked` is true, then this represents an assertion that needs to be
/// checked. Otherwise, it is an assumption.
pub struct Fact {
    prop: PropIdx,
    checked: bool,
}

impl Fact {
    /// An assertion which is required to be statically proved
    pub fn assert(prop: PropIdx) -> Self {
        Self {
            prop,
            checked: true,
        }
    }

    /// An assumption which is not checked
    pub fn assume(prop: PropIdx) -> Self {
        Self {
            prop,
            checked: false,
        }
    }
}

impl Foldable<ParamIdx, ExprIdx> for PropIdx {
    type Context = Component;

    fn fold_with<F>(&self, ctx: &mut Self::Context, subst_fn: &mut F) -> Self
    where
        F: FnMut(ParamIdx) -> Option<ExprIdx>,
    {
        match ctx.get(*self).clone() {
            Prop::True => *self,
            Prop::Cmp { op, lhs, rhs } => {
                let lhs = lhs.fold_with(ctx, subst_fn);
                let rhs = rhs.fold_with(ctx, subst_fn);
                ctx.add(Prop::Cmp { op, lhs, rhs })
            }
            Prop::Not(p) => {
                let p = p.fold_with(ctx, subst_fn);
                p.not(ctx)
            }
            Prop::And(l, r) => {
                let l = l.fold_with(ctx, subst_fn);
                let r = r.fold_with(ctx, subst_fn);
                l.and(r, ctx)
            }
            Prop::Or(l, r) => {
                let l = l.fold_with(ctx, subst_fn);
                let r = r.fold_with(ctx, subst_fn);
                l.or(r, ctx)
            }
            Prop::Implies(a, c) => {
                let a = a.fold_with(ctx, subst_fn);
                let c = c.fold_with(ctx, subst_fn);
                a.implies(c, ctx)
            }
        }
    }
}
