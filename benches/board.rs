use criterion::{black_box, criterion_group, criterion_main, Criterion};
use minesweeper::board::Board;
use minesweeper::data::Data;

fn get_board () -> Board {
    let data: Data = include_str!("complicatedboard.txt").parse().expect("unable to do conversion");
    Board::from_previous_data(data).expect("unable to convert from previous data")
}

fn criterion_benchmark(c: &mut Criterion) {
    let board = get_board();

    c.bench_function("render complicated board", |b| b.iter(|| {
        let rendered = board.render();
        black_box(rendered);
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);