use fil_ir::{self as ir, AddCtx};

#[derive(Default)]
/// Hoist all [ir::Fact] from the control flow graph into the top level by
/// adding their path conditions.
pub struct HoistFacts {
    /// Stack of path conditions
    stack: Vec<usize>,
    /// The current path condition
    path_cond: Vec<ir::PropIdx>,
    /// Facts to be hoisted
    facts: Vec<ir::Fact>,
}

impl HoistFacts {
    /// Push a new stack frame by tracking the number of added path conditions
    fn push(&mut self) {
        let len = self.path_cond.len();
        self.stack.push(len);
    }

    /// Pop the current stack frame
    fn pop(&mut self) {
        let props = self.stack.pop().unwrap();
        self.path_cond.truncate(props);
    }

    /// Insert a new path condition
    fn add_to_pc(&mut self, prop: ir::PropIdx) {
        self.path_cond.push(prop);
    }

    /// Return the current path condition
    fn path_cond(&mut self, ctx: &mut impl AddCtx<ir::Prop>) -> ir::PropIdx {
        let mut pc = ctx.add(ir::Prop::True);
        for prop in self.path_cond.iter().copied() {
            pc = pc.and(prop, ctx);
        }
        pc
    }

    // Parses a list of commands and hoists its assertions.
    fn cmds(&mut self, cmds: &Vec<ir::Command>, comp: &mut ir::Component) {
        // Collect all assumptions in a given scope and add them to the path condition.
        // We do this so that all asserts in a scope are affected by all assumes.
        for cmd in cmds {
            match cmd {
                ir::Command::Fact(f) => {
                    if f.is_assume() {
                        self.add_to_pc(f.prop)
                    }
                }
                ir::Command::Exists(ir::Exists { param, expr }) => {
                    let prop = param.expr(comp).equal(*expr, comp);
                    self.add_to_pc(prop);
                }
                _ => (),
            }
        }

        // Hoist all assertions in the current scope using the path condition.
        for cmd in cmds {
            match cmd {
                ir::Command::ForLoop(ir::Loop {
                    index,
                    start,
                    end,
                    body,
                }) => {
                    self.push();
                    let idx = index.expr(comp);
                    let start = idx.gte(*start, comp);
                    let end = idx.lt(*end, comp);
                    self.add_to_pc(start.and(end, comp));
                    self.cmds(body, comp);
                    self.pop()
                }
                ir::Command::If(i) => {
                    // Adds commands in if statement with condition predicate.
                    self.push();
                    self.add_to_pc(i.cond);
                    self.cmds(&i.then, comp);
                    self.pop();

                    self.push();
                    self.add_to_pc(i.cond.not(comp));
                    self.cmds(&i.alt, comp);
                    self.pop();
                }
                ir::Command::Fact(fact) => {
                    if fact.is_assert() {
                        // Otherwise this is a checked assertion that needs to be hoisted.
                        // Generate prop = path_cond -> fact.prop
                        let cond =
                            self.path_cond(comp).implies(fact.prop, comp);
                        match comp.assert(cond, fact.reason) {
                            Some(ir::Command::Fact(f)) => self.facts.push(f),
                            None => (),
                            _ => unreachable!(
                                "Tried to add non-fact in hoist-facts."
                            ),
                        }
                    }
                }
                _ => (),
            }
        }
    }

    /// Take a component and return all the hoisted facts to be proven
    pub fn hoist(comp: &mut ir::Component) -> Vec<ir::Fact> {
        let mut hoist = Self::default();

        // Need to take out the commands here so we can borrow comp mutably
        let mut cmds = std::mem::take(&mut comp.cmds);
        hoist.cmds(&cmds, comp);
        std::mem::swap(&mut comp.cmds, &mut cmds);

        assert!(hoist.stack.is_empty(), "Scopes were unbalanced!");

        hoist.facts
    }
}
