use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{Rng, thread_rng};
use coup_rs::Coup;

fn complete_game() {
    let mut rng = thread_rng();
    let mut coup = black_box(Coup::new(4));
    for _ in 0..1000 {
        let mut actions = coup.actions();

        let random_index = rng.gen_range(0..actions.len());
        let random_action = actions.remove(random_index);

        coup = coup.apply_action(random_action, &mut rng).unwrap();

        if let Some(_) = coup.winner() {
            break;
        }
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("complete game", |b| b.iter(|| complete_game()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);