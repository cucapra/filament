use crate::ast_visitor::{Action, Visitor};
use crate::cmdline;
use fil_ast as ast;
use fil_utils::{Diagnostics, Error, GPosIdx};
use std::collections::{HashMap, HashSet};

/// Structure to track usage information for a single component
#[derive(Default, Clone)]
struct ComponentUsage {
    // Definitions with their locations
    instances: HashMap<String, ast::Loc<fil_utils::Id>>,
    invocations: HashMap<String, ast::Loc<fil_utils::Id>>,
    let_params: HashMap<String, ast::Loc<fil_utils::Id>>,
    sig_params: HashMap<String, ast::Loc<fil_utils::Id>>,
    
    // Usage tracking
    invoked_instances: HashSet<String>,
    accessed_invocations: HashSet<String>,
    referenced_params: HashSet<String>,
}


/// Check for unused elements in AST (instances, invocations, parameters)
pub struct UnusedElementsCheck {
    diag: Diagnostics,
    current_component: Option<String>,
    component_usage: HashMap<String, ComponentUsage>,
    warnings_as_errors: bool,
}

impl crate::ast_visitor::Construct for UnusedElementsCheck {
    fn from(opts: &cmdline::Opts, _ast: &mut ast::Namespace) -> Self {
        UnusedElementsCheck {
            diag: Diagnostics::default(),
            current_component: None,
            component_usage: HashMap::new(),
            warnings_as_errors: opts.warnings_as_errors,
        }
    }

    fn clear_data(&mut self) {
        // Don't clear component_usage since we need it across components
        self.current_component = None;
    }
}

impl UnusedElementsCheck {
    fn current_usage(&mut self) -> &mut ComponentUsage {
        let comp_name = self.current_component.clone().expect("No current component");
        self.component_usage.entry(comp_name).or_default()
    }

    fn extract_invocation_name(&self, port: &ast::Port) -> Option<String> {
        match &port.base {
            ast::PortRef::Instance { instance, .. } => {
                Some(instance.inner().as_ref().to_string())
            }
            _ => None,
        }
    }

    fn visit_expr_for_params(&mut self, expr: &ast::Expr) {
        match expr {
            ast::Expr::Abstract(name) => {
                let usage = self.current_usage();
                usage.referenced_params.insert(name.inner().as_ref().to_string());
            }
            ast::Expr::ParamAccess { inst: _, param } => {
                let usage = self.current_usage();
                usage.referenced_params.insert(param.inner().as_ref().to_string());
            }
            ast::Expr::Op { left, right, .. } => {
                self.visit_expr_for_params(left);
                self.visit_expr_for_params(right);
            }
            ast::Expr::If { cond, then, alt } => {
                self.visit_order_constraint_for_params(cond);
                self.visit_expr_for_params(then);
                self.visit_expr_for_params(alt);
            }
            ast::Expr::App { args, .. } => {
                for arg in args {
                    self.visit_expr_for_params(arg);
                }
            }
            // Base cases: concrete values don't reference parameters
            ast::Expr::Concrete(_) => {}
        }
    }

    fn visit_order_constraint_for_params(&mut self, constraint: &ast::OrderConstraint<Box<ast::Expr>>) {
        self.visit_expr_for_params(&constraint.left);
        self.visit_expr_for_params(&constraint.right);
    }

    fn visit_order_constraint_unboxed_for_params(&mut self, constraint: &ast::OrderConstraint<ast::Expr>) {
        self.visit_expr_for_params(&constraint.left);
        self.visit_expr_for_params(&constraint.right);
    }

    fn visit_port_for_params(&mut self, port: &ast::Port) {
        // Visit access expressions in bundle accesses
        for access in &port.access {
            self.visit_expr_for_params(&access.start);
            self.visit_expr_for_params(&access.end);
        }
    }

    fn add_unused_warning(&mut self, element_type: &str, description: &str, loc: GPosIdx) {
        let mut warning = Error::unused_element(element_type, description)
            .add_note(self.diag.add_info(format!("{} defined here", element_type), loc));

        // Promote to error if flag is set
        if self.warnings_as_errors {
            warning = warning.as_error();
        }

        self.diag.add_error(warning);
    }

    fn check_component_unused(&mut self, comp_name: &str) {
        // Clone the usage data to avoid borrowing issues
        let usage = match self.component_usage.get(comp_name).cloned() {
            Some(u) => u,
            None => return, // No usage data for this component
        };

        // Collect all warnings first, then sort them for deterministic output
        let mut warnings = Vec::new();

        // Check unused instances
        for (inst_name, def_loc) in &usage.instances {
            if !usage.invoked_instances.contains(inst_name) {
                warnings.push((def_loc.pos(), "instance", "instance is created but never invoked"));
            }
        }

        // Check unused invocations (no ports accessed)
        for (inv_name, def_loc) in &usage.invocations {
            if !usage.accessed_invocations.contains(inv_name) {
                warnings.push((def_loc.pos(), "invocation", "output ports on invocation never used"));
            }
        }

        // Check unused let parameters
        for (param_name, def_loc) in &usage.let_params {
            if !usage.referenced_params.contains(param_name) {
                warnings.push((def_loc.pos(), "parameter", "parameter is defined but never used"));
            }
        }

        // Check unused signature parameters
        for (param_name, def_loc) in &usage.sig_params {
            if !usage.referenced_params.contains(param_name) {
                warnings.push((def_loc.pos(), "parameter", "parameter is defined but never used"));
            }
        }

        // Sort warnings by position for deterministic output
        // We can't access PosIdx internals, so sort by element type and description for determinism
        warnings.sort_by(|(pos_a, type_a, desc_a), (pos_b, type_b, desc_b)| {
            // First try to compare by type and description for determinism
            match (type_a, desc_a).cmp(&(type_b, desc_b)) {
                std::cmp::Ordering::Equal => {
                    // If types are the same, compare by position hash (not perfect but deterministic in practice)
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher_a = DefaultHasher::new();
                    let mut hasher_b = DefaultHasher::new();
                    pos_a.hash(&mut hasher_a);
                    pos_b.hash(&mut hasher_b);
                    hasher_a.finish().cmp(&hasher_b.finish())
                }
                other => other,
            }
        });

        // Generate warnings in sorted order
        for (pos, element_type, description) in warnings {
            self.add_unused_warning(element_type, description, pos);
        }
    }
}

impl Visitor for UnusedElementsCheck {
    fn name() -> &'static str {
        "unused-elements"
    }

    fn signature(&mut self, sig: &mut ast::Signature) -> Action {
        let comp_name = sig.name.inner().as_ref().to_string();
        self.current_component = Some(comp_name.clone());
        
        // Record signature parameters
        let usage = self.current_usage();
        for param in &sig.params {
            usage.sig_params.insert(
                param.name().as_ref().to_string(),
                param.param.clone(),
            );
        }

        // Record let-bound parameters from the 'with' section  
        // First pass: record parameters
        for sig_bind in &sig.sig_bindings {
            match sig_bind.inner() {
                ast::SigBind::Let { param, .. } => {
                    usage.let_params.insert(
                        param.inner().as_ref().to_string(),
                        param.clone(),
                    );
                }
                ast::SigBind::Exists { param, .. } => {
                    usage.let_params.insert(
                        param.inner().as_ref().to_string(),
                        param.clone(),
                    );
                }
            }
        }

        // Second pass: visit expressions for parameter usage (after usage is populated)
        for sig_bind in &sig.sig_bindings {
            match sig_bind.inner() {
                ast::SigBind::Let { bind, .. } => {
                    self.visit_expr_for_params(bind);
                }
                ast::SigBind::Exists { cons, .. } => {
                    for constraint in cons {
                        self.visit_order_constraint_unboxed_for_params(constraint.inner());
                    }
                }
            }
        }

        // Don't stop traversal - we need to visit the component body
        Action::Continue
    }

    fn instance(&mut self, inst: &mut ast::Instance) -> Action {
        let usage = self.current_usage();
        usage.instances.insert(
            inst.name.inner().as_ref().to_string(),
            inst.name.clone(),
        );

        // Visit parameter expressions for parameter usage
        for param_expr in &inst.params {
            self.visit_expr_for_params(param_expr.inner());
        }

        Action::Continue
    }

    fn invoke(&mut self, inv: &mut ast::Invoke) -> Action {
        let usage = self.current_usage();
        
        // Record invocation definition
        usage.invocations.insert(
            inv.name.inner().as_ref().to_string(),
            inv.name.clone(),
        );
        
        // Mark the instance as being invoked
        usage.invoked_instances.insert(
            inv.instance.inner().as_ref().to_string()
        );

        // Visit port expressions for parameter usage
        for port in &inv.ports {
            self.visit_port_for_params(port.inner());
        }

        Action::Continue
    }

    fn param_let(&mut self, param: &mut ast::ParamLet) -> Action {
        let usage = self.current_usage();
        usage.let_params.insert(
            param.name.inner().as_ref().to_string(),
            param.name.clone(),
        );

        // If the parameter has an expression, visit it for parameter references
        if let Some(expr) = &param.expr {
            self.visit_expr_for_params(expr);
        }

        Action::Continue
    }

    fn connect(&mut self, conn: &mut ast::Connect) -> Action {
        // Check if source is from an invocation and mark it as accessed
        if let Some(inv_name) = self.extract_invocation_name(conn.src.inner()) {
            let usage = self.current_usage();
            usage.accessed_invocations.insert(inv_name);
        }

        // Visit port expressions for parameter usage
        self.visit_port_for_params(conn.src.inner());
        self.visit_port_for_params(conn.dst.inner());

        Action::Continue
    }

    fn fact(&mut self, fact: &mut ast::Fact) -> Action {
        // Visit expressions in facts for parameter usage
        for expr in fact.exprs() {
            self.visit_expr_for_params(expr);
        }
        Action::Continue
    }

    fn exists(&mut self, exists: &mut ast::Exists) -> Action {
        // Visit the binding expression for parameter usage
        self.visit_expr_for_params(exists.bind.inner());
        Action::Continue
    }

    fn finish(&mut self, namespace: &mut ast::Namespace) {
        // Check each component for unused elements
        for comp in &namespace.components {
            let comp_name = comp.sig.name.inner().as_ref().to_string();
            self.check_component_unused(&comp_name);
        }
    }

    fn after_traversal(mut self) -> Option<u64> {
        self.diag.report_all()
    }
}
