use super::{idxs::PropIdx, Ctx, ExprIdx};

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
}

#[derive(Clone, PartialEq, Eq, Hash)]
/// A fact in the program.
/// If `checked` is true, then this represents an assertion that needs to be
/// checked. Otherwise, it is an assumption.
pub struct Fact {
    prop: PropIdx,
    checked: bool,
}
