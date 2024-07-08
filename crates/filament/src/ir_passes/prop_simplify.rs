use crate::ir_visitor::{Action, Visitor, VisitorData};
use fil_ir::{self as ir, AddCtx, Ctx};
use linked_hash_set::LinkedHashSet;

#[derive(Default)]
/// Simplify propositions and expressions in a component
pub struct Simplify {
    /// Simplified version of each proposition
    prop_map: ir::DenseIndexInfo<ir::Prop, ir::PropIdx>,
}

/// Simplify propositions
impl Simplify {
    /// Get the simplified version of a proposition if present.
    fn get_simpl(&self, prop: ir::PropIdx) -> ir::PropIdx {
        if self.prop_map.contains(prop) {
            self.prop_map[prop]
        } else {
            prop
        }
    }

    /// Get all the unique conjuncts of a proposition
    fn conjuncts(
        &self,
        prop: ir::PropIdx,
        c: &impl ir::Ctx<ir::Prop>,
        acc: &mut Vec<ir::PropIdx>,
    ) {
        match c.get(prop) {
            ir::Prop::And(l, r) => {
                // get_simpl call here because the constituent might have already
                // simplified
                self.conjuncts(self.get_simpl(*l), c, acc);
                self.conjuncts(self.get_simpl(*r), c, acc);
            }
            ir::Prop::True => (),
            _ => {
                acc.push(prop);
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
                self.disjuncts(self.get_simpl(*l), c, acc);
                self.disjuncts(self.get_simpl(*r), c, acc);
            }
            ir::Prop::False => (),
            _ => {
                acc.insert(prop);
            }
        }
    }

    fn prop_from_conjuncts<C>(
        conj: LinkedHashSet<ir::PropIdx>,
        ctx: &mut C,
    ) -> ir::PropIdx
    where
        C: Ctx<ir::Prop> + AddCtx<ir::Prop>,
    {
        // Negative atoms we've already seen
        let mut neg = LinkedHashSet::new();
        let mut acc = ctx.add(ir::Prop::True);
        for c in conj.into_iter() {
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

    fn prop_from_disjuncts<C>(
        disj: LinkedHashSet<ir::PropIdx>,
        ctx: &mut C,
    ) -> ir::PropIdx
    where
        C: Ctx<ir::Prop> + AddCtx<ir::Prop>,
    {
        // Negative atoms we've already seen
        let mut neg = LinkedHashSet::new();
        let mut acc = ctx.add(ir::Prop::False);
        for d in disj.into_iter() {
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

    fn simplify_prop(
        &mut self,
        prop: ir::PropIdx,
        ctx: &mut ir::Component,
    ) -> ir::PropIdx {
        if !self.prop_map.contains(prop) {
            let out = match ctx.get(prop).clone() {
                ir::Prop::True
                | ir::Prop::False
                | ir::Prop::Cmp(_)
                | ir::Prop::TimeCmp(_)
                | ir::Prop::TimeSubCmp(_) => prop,
                ir::Prop::Not(p) => self.simplify_prop(p, ctx).not(ctx),
                ir::Prop::And(_, _) => {
                    let mut conj = Vec::new();
                    self.conjuncts(prop, ctx, &mut conj);
                    let conj_len = conj.len();
                    let set: LinkedHashSet<_> = conj.into_iter().collect();
                    // Only perform the simplification if there are duplicates
                    if set.len() == conj_len {
                        prop
                    } else {
                        Self::prop_from_conjuncts(set, ctx)
                    }
                }
                ir::Prop::Or(_, _) => {
                    let mut disj = LinkedHashSet::new();
                    self.disjuncts(prop, ctx, &mut disj);
                    Self::prop_from_disjuncts(disj, ctx)
                }
                ir::Prop::Implies(a, c) => {
                    // Compute the conjuncts for the antecedant and consequent and eliminate terms in the consequent that are in the antecedant
                    // NOTE(rachit): The simplification of `a` would already have computed the conjuncts so we're wasting some work here.
                    let mut conj_a = Vec::new();
                    self.conjuncts(a, ctx, &mut conj_a);
                    let set_a = conj_a.into_iter().collect();

                    let mut conj_c = Vec::new();
                    self.conjuncts(c, ctx, &mut conj_c);
                    let set_c: LinkedHashSet<_> = conj_c.into_iter().collect();

                    let diff: LinkedHashSet<_> =
                        set_c.difference(&set_a).copied().collect();
                    if diff.len() == set_c.len() {
                        prop
                    } else {
                        let new_c = Self::prop_from_conjuncts(diff, ctx);
                        self.simplify_prop(a, ctx).implies(new_c, ctx)
                    }
                }
            };
            self.prop_map.push(prop, out)
        }
        self.prop_map[prop]
    }
}

impl Visitor for Simplify {
    fn name() -> &'static str {
        "simplify"
    }

    fn start(&mut self, data: &mut VisitorData) -> Action {
        let old_len = data.comp.props().size();
        // Populate the prop_map with the simplified version of each proposition.
        for prop in data.comp.props().idx_iter() {
            // Only simplify the old propositions
            if prop.get() >= old_len {
                break;
            }

            let out = self.simplify_prop(prop, &mut data.comp);
            if prop != out {
                log::debug!("{prop} ==> {out}");
            } else {
                log::debug!("{prop} unchanged");
            }
        }
        Action::Continue
    }

    fn fact(&mut self, fact: &mut ir::Fact, data: &mut VisitorData) -> Action {
        // Simplify the proposition in the fact
        let simpl = self.simplify_prop(fact.prop, &mut data.comp);
        if simpl.is_true(&data.comp) {
            Action::Change(vec![])
        } else {
            fact.prop = simpl;
            Action::Continue
        }
    }
}
