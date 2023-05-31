#[derive(Default)]
/// Filament's core interval checking algorithm. At a high-level it ensures that:
/// 1. All delays are well-formed
/// 2. Ports are connected for as long as expected
///
/// In order to ensure that delays are well-formed, we need to ensure that:
/// * Invocations provide events that trigger less often that expected by the
///   delay of the invoked component.
/// * The availability of bundle signals is less than the delay
/// * Shared instances are live for shorter duration than the delay
///
/// Like [super::TypeCheck], this pass simply generates all the assertions that
/// enforce the above constraints.
/// It is the job of a latter pass to ensure that the assertions are discharged.
pub struct IntervalCheck;
