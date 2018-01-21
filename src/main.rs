#![feature(entry_and_modify)]
#![feature(slice_patterns)]

extern crate itertools;
extern crate nalgebra as na;

mod aes;
mod codec;
mod pkcs;
mod s_box;
mod stats;
mod xor;

mod set_1;
mod set_2;

fn main() {
   set_1::set_1();
   set_2::set_2();
}
