#[macro_export]
/// Return the result of the computation and the time it took to run it.
macro_rules! time {
    ($e:expr) => {{
        let t = std::time::Instant::now();
        let r = $e;
        (r, t.elapsed())
    }};
}

#[macro_export]
/// Logs the amount of time it took to run the expression.
///
macro_rules! log_time {
    ($e:expr) => {{
        let (r, t) = $crate::time!($e);
        log::info!("{}: {}ms", stringify!($e), t.as_millis());
        r
    }};
    // Variant to log the time with a custom message.
    ($e:expr, $msg:expr) => {{
        let (r, t) = $crate::time!($e);
        log::info!("{}: {}ms", $msg, t.as_millis());
        r
    }};
    // Variant to log the time with a custom message only when a bound is reached
    ($e:expr, $msg:expr; $min_time:expr) => {{
        let (r, t) = $crate::time!($e);
        if t.as_millis() > $min_time {
            log::info!("{}: {}ms", $msg, t.as_millis());
        }
        r
    }};
}

#[macro_export]
/// A macro generate the pass pipeline. For each provided pass, it will:
/// 1. Record the amount of time it took to run the pass.
/// 2. Print out the state of the AST if the name of the pass was in the
///    print-after declaration.
///
/// Usage:
/// ```
/// pass_pipeline! { opts, ir;
///   Pass1,
///   Pass2, ...
/// }
macro_rules! pass_pipeline {
    ($opts:ident, $ir:ident; $($pass:path),*) => {
        $(
            let name = <$pass as $crate::ir_visitor::Visitor>::name();
            $crate::log_time!(<$pass as $crate::ir_visitor::Visitor>::do_pass($opts, &mut $ir)?, name);
            if $opts.dump_after.contains(&name.to_string()) {
                $crate::ir::Printer::context(& $ir, &mut std::io::stdout()).unwrap()
            }
        )*
    };
}
