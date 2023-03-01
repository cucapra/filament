use crate::utils::GPosIdx;

/// A type that contains several position objects
pub struct Loc<const N: usize, T> {
    inner: T,
    positions: [GPosIdx; N],
}
