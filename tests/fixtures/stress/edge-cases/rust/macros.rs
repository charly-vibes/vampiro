macro_rules! define_struct {
    ($name:ident, $field:ident: $ty:ty) => {
        struct $name {
            $field: $ty,
        }
    };
}

define_struct!(Point, x: i32);
define_struct!(Point, y: f64); // reuse name — harmless

fn main() {
    let _v = vec![1, 2, 3];
    let _s = stringify!(hello world);
    println!("{_s}");
}