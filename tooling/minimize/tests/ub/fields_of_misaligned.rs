#[repr(align(4))]
struct S {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    array1: [u16; 7],
    array2: [u16; 7],
}
use std::ptr::addr_of_mut;
fn main() { unsafe {
    let mut mem = [0u8; 128];
    let k = 4 - ((addr_of_mut!(mem[0]) as usize) % 4);

    // add 2 to get misaligned pointer that is still valid for indexing the arrays
    let i = (k+2) % 4;
    let ptr = addr_of_mut!(mem[i]) as *mut S;
    (*ptr).array1[5] = 0xABCD;
    (*ptr).array2[5] = 0xABCD;

    // However, this is UB because there is no indexing. The field access
    // requires the parent struct to be aligned normally
    //(*ptr).array1 = [0xFF; 7];

    // add 1 to get misaligned pointer
    let i = (k+1) % 4;
    let ptr = addr_of_mut!(mem[i]) as *mut S;

    // these are okay because taking the address "forgets" what was dereferenced
    // to get it
    *addr_of_mut!((*ptr).a) = 0xAA;
    *addr_of_mut!((*ptr).b) = 0xBB;
    *addr_of_mut!((*ptr).c) = 0xCC;
    *addr_of_mut!((*ptr).d) = 0xDD;


    // ptr is not aligned for S, so all of these should be UB:

    //(*ptr).a = 0xAA;
    //(*ptr).b = 0xBB;
    //(*ptr).c = 0xCC;
    //(*ptr).d = 0xDD;
    (*ptr).array2[5] = 0xABCD;

}}
