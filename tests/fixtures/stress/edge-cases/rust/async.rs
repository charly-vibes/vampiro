use std::future::Future;

async fn fetch() -> u32 {
    42
}

fn spawn<F: Future<Output = ()> + Send + 'static>(f: F) {
    // pretend runtime
}

fn main() {
    let fut = fetch();
    let _: &dyn Future<Output = u32> = &fut;
    spawn(async {
        let x = fetch().await;
        println!("{x}");
    });
}