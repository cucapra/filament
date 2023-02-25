use egg::{
    self, define_language, rewrite, AstSize, Extractor, Id, RecExpr, Rewrite,
    Runner, Symbol,
};

define_language! {
    enum ExprLang {
        Num(u64),
        Abstract(Symbol),
        "+" = Add([Id; 2]),
        "-" = Sub([Id; 2]),
        "*" = Mul([Id; 2]),
        "/" = Div([Id; 2]),
    }
}

fn make_rules() -> Vec<Rewrite<ExprLang, ()>> {
    vec![
        // Commute rules
        rewrite!("commute-add"; "(+ ?a ?b)" => "(+ ?b ?a)"),
        rewrite!("commute-mul"; "(* ?a ?b)" => "(* ?b ?a)"),
        // Identity rules
        rewrite!("add-0"; "(+ ?a 0)" => "?a"),
        rewrite!("sub-0"; "(- ?a 0)" => "?a"),
        rewrite!("mul-1"; "(* ?a 1)" => "?a"),
        rewrite!("div-1"; "(/ ?a 1)" => "?a"),
        // Absorption rules
        rewrite!("mul-0"; "(* ?a 0)" => "0"),
        rewrite!("div-0"; "(/ 0 ?a)" => "0"),
        rewrite!("sub-inv"; "(- ?a ?a)" => "0"),
        // Inverse rules
        rewrite!("add-neg"; "(+ ?a (- ?a ?b))" => "?b"),
        rewrite!("add-neg-rev"; "(- (+ ?a ?b) ?a)" => "?b"),
        // Distributive rules
        rewrite!("mul-div"; "(* ?a (/ ?b ?c))" => "(* (/ ?a ?c) ?b)"),
        rewrite!("distrib-mul-add"; "(* ?a (+ ?b ?c))" => "(+ (* ?a ?b) (* ?a ?c))"),
        rewrite!("distrib-mul-sub"; "(* ?a (- ?b ?c))" => "(- (* ?a ?b) (* ?a ?c))"),
        rewrite!("distrib-div-add"; "(/ (+ ?a ?b) ?c)" => "(+ (/ ?a ?c) (/ ?b ?c))"),
        rewrite!("distrib-div-sub"; "(/ (- ?a ?b) ?c)" => "(- (/ ?a ?c) (/ ?b ?c))"),
    ]
}

pub struct Simplifier;

impl Simplifier {
    /// parse an expression, simplify it using egg, and pretty print it back out
    pub fn simplify(s: &str) -> String {
        // parse the expression, the type annotation tells it which Language to use
        let expr: RecExpr<ExprLang> = s.parse().unwrap();

        // simplify the expression using a Runner, which creates an e-graph with
        // the given expression and runs the given rules over it
        let runner = Runner::default().with_expr(&expr).run(&make_rules());

        // the Runner knows which e-class the expression given with `with_expr` is in
        let root = runner.roots[0];

        // use an Extractor to pick the best element of the root eclass
        let extractor = Extractor::new(&runner.egraph, AstSize);
        let (best_cost, best) = extractor.find_best(root);
        println!("Simplified {} to {} with cost {}", expr, best, best_cost);
        best.to_string()
    }
}
