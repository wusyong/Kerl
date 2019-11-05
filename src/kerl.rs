use crate::keccak::Keccak;
use crate::Sponge;
use crate::constants::*;

#[derive(Clone, Copy)]
pub struct Kerl(Keccak);

impl Default for Kerl {
    fn default() -> Kerl {
        Kerl(Keccak::new_keccak384())
    }
}

impl Sponge for Kerl
where
    Self: Send + 'static,
{
    type Item = Trit;

    fn absorb(&mut self, trits: &[Self::Item]) {
        assert_eq!(trits.len() % TRIT_LENGTH, 0);
        let mut bytes: [u8; BYTE_LENGTH] = [0; BYTE_LENGTH];

        for chunk in trits.chunks(TRIT_LENGTH) {
            trits_to_bytes(chunk, &mut bytes);
            self.0.update(&bytes);
        }
    }

    fn squeeze(&mut self, out: &mut [Self::Item]) {
        assert_eq!(out.len() % TRIT_LENGTH, 0);
        let mut bytes: [u8; BYTE_LENGTH] = [0; BYTE_LENGTH];

        for chunk in out.chunks_mut(TRIT_LENGTH) {
            self.0.pad();
            self.0.fill_block();
            self.0.squeeze(&mut bytes);
            self.reset();
            bytes_to_trits(&mut bytes.to_vec(), chunk);
            for b in bytes.iter_mut() {
                *b = *b ^ 0xFF;
            }
            self.0.update(&bytes);
        }
    }

    fn reset(&mut self) {
        self.0 = Keccak::new_keccak384();
    }
}

fn trits_to_bytes(trits: &[Trit], bytes: &mut [u8]) {
    assert_eq!(trits.len(), TRIT_LENGTH);
    assert_eq!(bytes.len(), BYTE_LENGTH);

    // We _know_ that the sizes match.
    // So this is safe enough to do and saves us a few allocations.
    let base: &mut [u32] =
        unsafe { core::slice::from_raw_parts_mut(bytes.as_mut_ptr() as *mut u32, 12) };

    base.clone_from_slice(&[0; 12]);

    let mut size = 1;
    let mut all_minus_1 = true;

    for t in trits[0..TRIT_LENGTH - 1].iter() {
        if *t != -1 {
            all_minus_1 = false;
            break;
        }
    }

    if all_minus_1 {
        base.clone_from_slice(&HALF_3);
        bigint_not(base);
        bigint_add_small(base, 1_u32);
    } else {
        for t in trits[0..TRIT_LENGTH - 1].iter().rev() {
            // multiply by radix
            {
                let sz = size;
                let mut carry: u32 = 0;

                for j in 0..sz {
                    let v = (base[j] as u64) * (RADIX as u64) + (carry as u64);
                    let (newcarry, newbase) = ((v >> 32) as u32, v as u32);
                    carry = newcarry;
                    base[j] = newbase;
                }

                if carry > 0 {
                    base[sz] = carry;
                    size += 1;
                }
            }

            let trit = (t + 1) as u32;
            // addition
            {
                let sz = bigint_add_small(base, trit);
                if sz > size {
                    size = sz;
                }
            }
        }

        if !is_null(base) {
            if bigint_cmp(&HALF_3, base) <= 0 {
                // base >= HALF_3
                // just do base - HALF_3
                bigint_sub(base, &HALF_3);
            } else {
                // we don't have a wrapping sub.
                // so let's use some bit magic to achieve it
                let mut tmp = HALF_3.clone();
                bigint_sub(&mut tmp, base);
                bigint_not(&mut tmp);
                bigint_add_small(&mut tmp, 1_u32);
                base.clone_from_slice(&tmp);
            }
        }
    }

    bytes.reverse();
}

    /// This will consume the input bytes slice and write to trits.
fn bytes_to_trits(bytes: &mut [u8], trits: &mut [Trit]) {
    assert_eq!(bytes.len(), BYTE_LENGTH);
    assert_eq!(trits.len(), TRIT_LENGTH);

    trits[TRIT_LENGTH - 1] = 0;

    bytes.reverse();
    // We _know_ that the sizes match.
    // So this is safe enough to do and saves us a few allocations.
    let base: &mut [u32] =
        unsafe { core::slice::from_raw_parts_mut(bytes.as_mut_ptr() as *mut u32, 12) };

    if is_null(base) {
        trits.clone_from_slice(&[0; TRIT_LENGTH]);
        return;
    }

    let mut flip_trits = false;

    if base[INT_LENGTH - 1] >> 31 == 0 {
        // positive number
        // we need to add HALF_3 to move it into positvie unsigned space
        bigint_add(base, &HALF_3);
    } else {
        // negative number
        bigint_not(base);
        if bigint_cmp(base, &HALF_3) > 0 {
            bigint_sub(base, &HALF_3);
            flip_trits = true;
        } else {
            bigint_add_small(base, 1 as u32);
            let mut tmp = HALF_3.clone();
            bigint_sub(&mut tmp, base);
            base.clone_from_slice(&tmp);
        }
    }

    let mut rem;
    for i in 0..TRIT_LENGTH - 1 {
        rem = 0;
        for j in (0..INT_LENGTH).rev() {
            let lhs = ((rem as u64) << 32) | (base[j] as u64);
            let rhs = RADIX as u64;
            let q = (lhs / rhs) as u32;
            let r = (lhs % rhs) as u32;

            base[j] = q;
            rem = r;
        }
        trits[i] = rem as i8 - 1;
    }

    if flip_trits {
        for v in trits.iter_mut() {
            *v = -*v;
        }
    }
}

fn bigint_not(base: &mut [u32]) {
    for i in base.iter_mut() {
        *i = !*i;
    }
}

fn bigint_add_small(base: &mut [u32], other: u32) -> usize {
    let (mut carry, v) = full_add(base[0], other, false);
    base[0] = v;

    let mut i = 1;
    while carry {
        let (c, v) = full_add(base[i], 0, carry);
        base[i] = v;
        carry = c;
        i += 1;
    }

    i
}

fn bigint_add(base: &mut [u32], rh: &[u32]) {
    let mut carry = false;

    for (a, b) in base.iter_mut().zip(rh.iter()) {
        let (c, v) = full_add(*a, *b, carry);
        *a = v;
        carry = c;
    }
}

fn bigint_cmp(lh: &[u32], rh: &[u32]) -> i8 {
    for (a, b) in lh.iter().rev().zip(rh.iter().rev()) {
        if a < b {
            return -1;
        } else if a > b {
            return 1;
        }
    }
    return 0;
}

fn bigint_sub(base: &mut [u32], rh: &[u32]) {
    let mut noborrow = true;
    for (a, b) in base.iter_mut().zip(rh) {
        let (c, v) = full_add(*a, !*b, noborrow);
        *a = v;
        noborrow = c;
    }
    assert!(noborrow);
}

fn is_null(base: &[u32]) -> bool {
    for b in base.iter() {
        if *b != 0 {
            return false;
        }
    }
    return true;
}

fn full_add(lh: u32, rh: u32, carry: bool) -> (bool, u32) {
    let a = u64::from(lh);
    let b = u64::from(rh);

    let mut v = a + b;
    let mut l = v >> 32;
    let mut r = v & 0xFFFF_FFFF;

    let carry1 = l != 0;

    if carry {
        v = r + 1;
    }
    l = (v >> 32) & 0xFFFF_FFFF;
    r = v & 0xFFFF_FFFF;
    let carry2 = l != 0;
    (carry1 || carry2, r as u32)
}
