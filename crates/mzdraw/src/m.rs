//! Maths that must give the same bits in the `.wasm`, the app and the CLI: the trigonometry comes from the `libm` crate on every
//! target (the platform's own differs in the last digit), the rest is exact IEEE arithmetic.

pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}

pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}

pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}

pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

pub fn hypot(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// Sorts in place with `less` (a strict total order), by heap sort: O(n log n) on any input and a few hundred bytes of `.wasm`
/// per element type, where each use of the sorts of the standard library costs about 3 kB. Not stable: the callers sort by a total
/// order, so equal elements cannot be told apart.
pub fn sort_by<T>(v: &mut [T], less: impl Fn(&T, &T) -> bool) {
    let sift = |v: &mut [T], mut root: usize, end: usize| loop {
        let mut child = 2 * root + 1;
        if child >= end {
            break;
        }
        if child + 1 < end && less(&v[child], &v[child + 1]) {
            child += 1;
        }
        if !less(&v[root], &v[child]) {
            break;
        }
        v.swap(root, child);
        root = child;
    };
    for i in (0..v.len() / 2).rev() {
        sift(v, i, v.len());
    }
    for end in (1..v.len()).rev() {
        v.swap(0, end);
        sift(v, 0, end);
    }
}

/// `sort_by` with the order of the type.
pub fn sort<T: PartialOrd>(v: &mut [T]) {
    sort_by(v, |a, b| a < b);
}

#[cfg(test)]
mod tests {
    #[test]
    fn heap_sort_sorts_like_the_standard_library() {
        let mut x = 0x9E37_79B9_7F4A_7C15u64;
        for n in 0..200usize {
            let mut v: Vec<(u32, u32)> = (0..n)
                .map(|_| {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    ((x % 17) as u32, (x >> 40) as u32 % 5)
                })
                .collect();
            let mut want = v.clone();
            want.sort();
            super::sort(&mut v);
            assert_eq!(v, want, "{n} elements");
        }
        let mut f = vec![2.5, -0.0, 0.0, -1.0, f64::INFINITY, 1e-9];
        super::sort_by(&mut f, |a, b| a.total_cmp(b).is_lt());
        assert_eq!(f, [-1.0, -0.0, 0.0, 1e-9, 2.5, f64::INFINITY]);
        assert!(f[1].is_sign_negative() && f[2].is_sign_positive());
    }
}
