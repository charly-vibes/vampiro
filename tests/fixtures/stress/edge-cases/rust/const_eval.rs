const FACTOR: u64 = 100;
const COMPUTED: u64 = {
    let mut x = 1;
    x += FACTOR;
    x
};

static STATIC_VAL: u64 = COMPUTED;

const fn add(a: u32, b: u32) -> u32 { a + b }

const RESULT: u32 = add(40, 2);

fn main() {
    let _ = COMPUTED;
    let _ = STATIC_VAL;
    let _ = RESULT;
}