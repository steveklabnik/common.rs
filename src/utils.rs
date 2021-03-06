//! Crypto utils
use std::intrinsics;
use std::mem;
use std::num;
use std::ptr;
use std::rand::os::OsRng;
use std::slice::MutableSlice;


/// `Bytes` to `u32` little-endian decoding.
pub fn u8to32_le(val: &mut u32, buf: &[u8]) {
    assert!(buf.len() >= 4);
    for i in range(0u, 4) {
        *val |= buf[i] as u32 << (8 * i);
    }
}

/// `u32` to `bytes` little-endian encoding.
pub fn u32to8_le(buf: &mut [u8], val: &u32) {
    assert!(buf.len() >= 4);
    for i in range(0u, 4) {
        buf[i] = (*val >> (8 * i)) as u8 & 0xff;
    }
}

/// `Bytes` to `u64` little-endian decoding.
pub fn u8to64_le(val: &mut u64, buf: &[u8]) {
    assert!(buf.len() >= 8);
    for i in range(0u, 8) {
        *val |= *buf.get(i).unwrap() as u64 << (8 * i);
    }
}

/// `u64` to `bytes` little-endian encoding.
pub fn u64to8_le(buf: &mut [u8], val: &u64) {
    assert!(buf.len() >= 8);
    for i in range(0u, 8) {
        buf[i] = (*val >> (8 * i)) as u8;
    }
}


/// Pad to multiple of 16.
///
/// Return `n` padding bytes to make `len + n` the shortest multiple of 16.
/// `n` is comprised between `0` and `15`.
pub fn pad16(len: uint) -> Vec<u8> {
    Vec::from_elem((16 - (len % 16)) % 16, 0)
}


/// Zero-out memory buffer.
pub fn zero_memory<T>(b: &mut [T]) {
    unsafe {
        // FIXME: not sure how much this llvm intrinsics could not be
        // optimized-out, maybe it would be better to use memset_s.
        intrinsics::volatile_set_memory(b.as_mut_ptr(), 0, b.len());
    }
}

/// Copy memory buffer.
///
/// Copy `count` elements from slice `src` to mutable slice `dst`.
/// Requirement: `count >= min(srclen, dstlen)`.
pub fn copy_slice_memory<T>(dst: &mut[T], src: &[T], count: uint) {
    assert!(dst.len() >= count && src.len() >= count);
    unsafe {
        ptr::copy_nonoverlapping_memory(dst.as_mut_ptr(),
                                        src.as_ptr(),
                                        count);
    }
}


// Return 1 iff x == y; 0 otherwise.
fn byte_eq(x: u8, y: u8) -> u8 {
    let mut z: u8 = !(x ^ y);
    z &= z >> 4;
    z &= z >> 2;
    z &= z >> 1;
    z
}

/// Compare bytes buffers.
///
/// Return `true` iff `x == y`; `false` otherwise.
pub fn bytes_eq<T>(x: &[T], y: &[T]) -> bool {
    if x.len() != y.len() {
        return false;
    }

    let size = x.len() * mem::size_of::<T>();
    let px = x.as_ptr() as *const u8;
    let py = y.as_ptr() as *const u8;

    let mut d: u8 = 0;
    unsafe {
        for i in range(0u, size) {
            d |= *px.offset(i as int) ^ *py.offset(i as int);
        }
    }

    // Would prefer to return the result of byte_eq() instead of making
    // this last comparison, but this function is called from contexts where
    // boolean values are explicitly expected and this comparison seems
    // the only way to convert to a bool in rust.
    byte_eq(d, 0) == 1
}

/// Conditionally swap bytes.
///
/// `x` and `y` are swapped iff `cond` is equal to `1`, there are left
/// unchanged iff `cond` is equal to `0`. Currently only works for arrays
/// of signed integers. `cond` is expected to be `0` or `1`.
pub fn bytes_cswap<T: Signed + Primitive + Int>(cond: T,
                                                x: &mut [T],
                                                y: &mut [T]) {
    assert_eq!(x.len(), y.len());

    let c: T = !(cond - num::one());
    for i in range(0u, x.len()) {
        let t = c & (x[i] ^ y[i]);
        x[i] = x[i] ^ t;
        y[i] = y[i] ^ t;
    }
}


/// Instanciate a secure RNG based on `urandom`.
pub fn urandom_rng() -> OsRng {
    OsRng::new().unwrap()
}


#[cfg(test)]
mod tests {
    use std::path::BytesContainer;
    use std::rand::random;

    use utils;


    #[test]
    fn test_zero_memory() {
        struct Test {
            x: [u32, ..16],
        };

        let one = [1u32, ..16];
        let zero = [0u32, ..16];
        let mut s = Test {x: one};
        assert!(s.x == one);
        super::zero_memory(s.x[mut]);
        assert!(s.x == zero);
    }

    #[test]
    fn test_byte_eq() {
        for _ in range(0u, 256) {
            let a: u8 = random();
            let b: u8 = random();
            assert_eq!(super::byte_eq(a, b) == 1, a == b);
        }
    }

    #[test]
    fn test_bytes_eq() {
        let a: [u8, ..64] = [0u8, ..64];
        let b: [u8, ..64] = [0u8, ..64];
        assert!(super::bytes_eq(a, b));

        for _ in range(0u, 256) {
            let va = Vec::from_fn(64, |_| random::<u8>());
            let a = va.container_as_bytes();
            let vb = Vec::from_fn(64, |_| random::<u8>());
            let b = vb.container_as_bytes();
            assert_eq!(super::bytes_eq(a, b), a == b);
        }
    }

    #[test]
    fn test_bytes_cswap() {
        let mut a1: [i8, ..64] = [0i8, ..64];
        let a2 = a1;
        let mut b1: [i8, ..64] = [1i8, ..64];
        let b2 = b1;

        utils::bytes_cswap(0, a1, b1);
        assert!(a1 == a2);
        assert!(b1 == b2);

        utils::bytes_cswap(1, a1, b1);
        assert!(a1 == b2);
        assert!(b1 == a2);
    }

    #[test]
    fn test_copy_slice() {
        let a: [i64, ..64] = [42, ..64];
        let mut b: [i64, ..64] = [0, ..64];

        assert!(a != b);
        utils::copy_slice_memory(b[mut], a[], a.len());
        assert!(a == b);
    }

    #[test]
    fn test_pad16() {
        assert!(utils::pad16(0).len() == 0);
        assert!(utils::pad16(16).len() == 0);
        assert!(utils::pad16(1).len() == 15);
        assert!(utils::pad16(15).len() == 1);
        assert!(utils::pad16(42).len() == 6);
    }
}
