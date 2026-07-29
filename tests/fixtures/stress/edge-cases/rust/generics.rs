trait Trait {
    type Output;
    fn process(&self) -> Self::Output;
}

impl<T: Clone> Trait for Vec<T> {
    type Output = Vec<T>;
    fn process(&self) -> Self::Output {
        self.clone()
    }
}

struct Wrapper<T: ?Sized>(T);

fn id<T>(x: T) -> T { x }

fn main() {
    let v: Vec<i32> = vec![1, 2, 3];
    let _ = v.process();
    let _ = id(42u64);
    let _: Wrapper<dyn Send> = unsafe { std::mem::zeroed() };
}