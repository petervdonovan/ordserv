use std::collections::HashMap;

/// # Safety
///
/// This function is safe to call from C.
#[no_mangle]
pub unsafe extern "C" fn add(left: usize, right: usize) -> usize {
    let mut h = HashMap::new();
    for i in 0..left {
        h.insert(i, i);
    }
    for i in 0..right {
        h.insert(i, i);
    }
    h.iter().map(|(_, v)| *v).sum()
}
