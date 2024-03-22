use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rand::{Rng, thread_rng};
use coup_rs::Coup;

fn complete_game(num_players: u8) {
    let mut rng = thread_rng();
    let mut coup = black_box(Coup::new(num_players, &mut rng));
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
    let mut group = c.benchmark_group("complete_game");
    for num_players in 3..=6u8 {
        group.bench_with_input(BenchmarkId::from_parameter(num_players), &num_players, |b, &num_players| {
            b.iter(|| complete_game(num_players))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);