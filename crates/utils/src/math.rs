/// Convert a n-dimensional array's index into a concrete number into an index into a flat array
pub fn flat_idx(indices: &[u64], lens: &[u64]) -> u64 {
    indices
        .iter()
        .enumerate()
        .map(|(i, idx)| idx * lens[i + 1..].iter().product::<u64>())
        .sum()
}

/// For a array of ranges, return all the indices generated in that range.
/// For example, if we have the input: [(0, 2), (0, 3)] then we get:
/// [ [0, 0], [0, 1], [0, 2], [1, 0], [1, 1], [1, 2] ]
pub fn all_indices(ranges: Vec<(u64, u64)>) -> Vec<Vec<u64>> {
    let mut indices = vec![vec![]];
    for (start, end) in ranges {
        indices = indices
            .into_iter()
            .flat_map(|idx| {
                (start..end).map(move |i| {
                    let mut x = idx.clone();
                    x.push(i);
                    x
                })
            })
            .collect();
    }
    indices
}

/// Convert a concrete number into an n-dimensional array's index
pub fn nd_idx(v: u64, lens: &[u64]) -> Vec<u64> {
    let mut idxs = Vec::with_capacity(lens.len());
    let mut v = v;
    for l in lens.iter().rev() {
        idxs.push(v % l);
        v /= l;
    }
    // We've computed idxs in reverse order
    idxs.reverse();
    idxs
}
