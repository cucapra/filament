use itertools::Itertools;
use linked_hash_set::LinkedHashSet;

use crate::ir::{self, Ctx};
/// Simplify proposition in a component
pub struct PropSimplify {
    /// Simplified version of each proposition
    prop_map: ir::DenseIndexInfo<ir::Prop, ir::PropIdx>,
}

impl PropSimplify {
    /// Get all the unique conjuncts of a proposition
    fn conjuncts(
        &self,
        prop: ir::PropIdx,
        c: &impl ir::Ctx<ir::Prop>,
        acc: &mut LinkedHashSet<ir::PropIdx>,
    ) {
        match c.get(prop) {
            ir::Prop::And(l, r) => {
                // The subterms are guaranteed to exist in the prop_map because
                // we simplify smaller terms before larger ones
                self.conjuncts(self.prop_map[*l], c, acc);
                self.conjuncts(self.prop_map[*r], c, acc);
            }
            ir::Prop::True => (),
            _ => {
                acc.insert(prop);
            }
        }
    }

    /// Get all the unique disjuncts of a proposition
    fn disjuncts(
        &self,
        prop: ir::PropIdx,
        c: &impl ir::Ctx<ir::Prop>,
        acc: &mut LinkedHashSet<ir::PropIdx>,
    ) {
        match c.get(prop) {
            ir::Prop::Or(l, r) => {
                self.disjuncts(self.prop_map[*l], c, acc);
                self.disjuncts(self.prop_map[*r], c, acc);
            }
            ir::Prop::False => (),
            _ => {
                acc.insert(prop);
            }
        }
    }

    fn prop_from_conjuncts(
        conj: LinkedHashSet<ir::PropIdx>,
        ctx: &mut impl ir::Ctx<ir::Prop>,
    ) -> ir::PropIdx {
        // Negative atoms we've already seen
        let mut neg = LinkedHashSet::new();
        let mut acc = ctx.add(ir::Prop::True);
        for c in conj.into_iter().sorted() {
            match ctx.get(c) {
                ir::Prop::False => return c,
                ir::Prop::Not(p) => {
                    neg.insert(*p);
                }
                _ => (),
            }
            // If we've already seen the negation of this atom, then the
            // conjunction is false
            if neg.contains(&c) {
                return ctx.add(ir::Prop::False);
            }
            acc = acc.and(c, ctx);
        }
        acc
    }

    fn prop_from_disjuncts(
        disj: LinkedHashSet<ir::PropIdx>,
        ctx: &mut impl ir::Ctx<ir::Prop>,
    ) -> ir::PropIdx {
        // Negative atoms we've already seen
        let mut neg = LinkedHashSet::new();
        let mut acc = ctx.add(ir::Prop::False);
        for d in disj.into_iter().sorted() {
            match ctx.get(d) {
                ir::Prop::True => return d,
                ir::Prop::Not(p) => {
                    neg.insert(*p);
                }
                _ => (),
            }
            // If we've already seen the negation of this atom, then the
            // disjunction is true
            if neg.contains(&d) {
                return ctx.add(ir::Prop::True);
            }
            acc = acc.or(d, ctx);
        }
        acc
    }

    fn simplify(
        &mut self,
        prop: ir::PropIdx,
        ctx: &mut ir::Component,
    ) -> ir::PropIdx {
        if !self.prop_map.contains(prop) {
            let out = match ctx.get(prop).clone() {
                ir::Prop::True | ir::Prop::False => prop,
                ir::Prop::Cmp(_) => todo!(),
                ir::Prop::TimeCmp(_) => todo!(),
                ir::Prop::TimeSubCmp(_) => todo!(),
                ir::Prop::Not(p) => self.simplify(p, ctx).not(ctx),
                ir::Prop::And(_, _) => {
                    let mut conj = LinkedHashSet::new();
                    self.conjuncts(prop, ctx, &mut conj);
                    Self::prop_from_conjuncts(conj, ctx)
                }
                ir::Prop::Or(_, _) => {
                    let mut disj = LinkedHashSet::new();
                    self.disjuncts(prop, ctx, &mut disj);
                    Self::prop_from_disjuncts(disj, ctx)
                }
                ir::Prop::Implies(a, c) => {
                    // Compute the conjuncts for the antecedant and consequent and eliminate terms in the consequent that are in the antecedant
                    // NOTE(rachit): The simplification of `a` would already have computed the conjuncts so we're wasting some work here.
                    let mut conj_a = LinkedHashSet::new();
                    self.conjuncts(a, ctx, &mut conj_a);

                    let mut conj_c = LinkedHashSet::new();
                    self.conjuncts(c, ctx, &mut conj_c);
                    let new_c = Self::prop_from_conjuncts(
                        conj_c.difference(&conj_a).copied().collect(),
                        ctx,
                    );

                    self.simplify(a, ctx).implies(new_c, ctx)
                }
            };
            self.prop_map.push(prop, out)
        }
        self.prop_map[prop]
    }
}
