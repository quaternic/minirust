use std::ptr::addr_of_mut;

#[repr(C,align(4))]
struct S {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    array1: [u16; 7],
    array2: [u16; 7],
}

fn main() { unsafe {
    let mut mem = [0u8; 128];
    let k = 4 - ((addr_of_mut!(mem[0]) as usize) % 4);

    // ptr is aligned for S, so these are OK
    let ptr = addr_of_mut!(mem[k]) as *mut S;
    (*ptr).a = 0xAA;
    (*ptr).b = 0xBB;
    (*ptr).c = 0xCC;
    (*ptr).d = 0xDD;
    (*ptr).array1[5] = 0xABCD;
    (*ptr).array2[5] = 0xABCD;

    // add 1 to get misaligned pointer
    let i = (k+1) % 4;
    let ptr = addr_of_mut!(mem[i]) as *mut S;

    // these are still okay because taking the address "forgets" what was dereferenced to get it
    *addr_of_mut!((*ptr).a) = 0xAA;
    *addr_of_mut!((*ptr).b) = 0xBB;
    *addr_of_mut!((*ptr).c) = 0xCC;
    *addr_of_mut!((*ptr).d) = 0xDD;

    // add 2 to get a pointer misaligned for S, but still aligned for the arrays
    let i = (k+2) % 4;
    let ptr = addr_of_mut!(mem[i]) as *mut S;

    // these accesses are okay because the base pointer is allowed to be misaligned
    // in bits that can also be changed by varying the index
    (*ptr).array1[5] = 0xABCD;
    (*ptr).array2[5] = 0xABCD;

    // See ../ub/fields_of_misaligned.rs for the UB version
}}
