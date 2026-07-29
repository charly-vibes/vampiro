// Unicode identifiers (Rust allows some Unicode in identifiers)
fn café() -> u32 { 42 }

struct Résumé {
    name: String,
    ɣ: f64, // Greek gamma
}

fn main() {
    let π = 3.14159f64;
    let _r = Résumé { name: "hello".into(), ɣ: π };
    let _ = café();
}