use bitflags::bitflags;

bitflags! {
    struct Flags: u8 {
            const FTEXT      = 1 <<  0;
            const FHCRC      = 1 <<  1;
            const FEXTRA     = 1 <<  2;
            const FNAME      = 1 <<  3;
            const FCOMMENT   = 1 <<  4;
    }
}

fn main() {
    println!("Hello, world!");
}
