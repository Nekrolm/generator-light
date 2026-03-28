// Simple benchmark to measure ovehread for generator codegen

use std::hint::black_box;
use std::pin::pin;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use generator_light::Generator;
use generator_light::GeneratorState;
use generator_light::Yielder;
use generator_light::ext::GeneratorExt;
use generator_light::ext::GeneratorIterator;
use generator_light::ext::from_fn;
use generator_light::ext::from_iter;
use generator_light::generator;

fn std_iter_func(n: usize) -> impl IntoIterator<Item = usize> {
    let mut idx = 1;
    std::iter::from_fn(move || {
        let cur = black_box(idx);
        // let cur = idx;
        let ret = (cur <= n).then_some(cur * cur);
        idx += 1;
        ret
    })
}

fn squares(n: usize) -> impl Generator<Yield = usize, Return = ()> {
    generator(async move |mut yilder: Yielder<_, _>| {
        for idx in 1..=n {
            let idx = black_box(idx);
            generator_light::yield_!(yilder, idx * idx)
        }
    })
}

fn squares_genawaiter(n: usize) {
    use genawaiter::yield_;
    genawaiter::stack::let_gen!(squares, {
        for idx in 1..=n {
            let idx = black_box(idx);
            yield_!(idx * idx);
        }
    });
    consume_iter(squares.into_iter());
}

fn squares_compose(n: usize) -> impl Generator<Yield = usize, Return = ()> {
    from_iter(1..)
        .compose(from_fn(move |x: usize| {
            let x = black_box(x);
            (x <= n)
                .then_some(GeneratorState::Yield(x * x))
                .unwrap_or(GeneratorState::Complete(()))
        }))
        .map_complete(drop)
}

fn consume_iter<I: Iterator<Item = usize>>(iter: I) {
    let mut s: usize = 0;
    for v in iter {
        s = s.wrapping_add(black_box(v));
    }
    black_box(s);
}

fn bench_generators(c: &mut Criterion) {
    let mut group = c.benchmark_group("generators");
    for &n in &[100_000] {
        group.bench_with_input(BenchmarkId::new("std_iter", n), &n, |b, &n| {
            b.iter(|| {
                let it = std_iter_func(n);
                consume_iter(it.into_iter());
            })
        });

        group.bench_with_input(BenchmarkId::new("squares_gen", n), &n, |b, &n| {
            b.iter(|| {
                let g = pin!(squares(n));
                consume_iter(g.into_iter());
            })
        });

        group.bench_with_input(BenchmarkId::new("squares_genawaiter", n), &n, |b, &n| {
            b.iter(|| {
                squares_genawaiter(n);
            })
        });

        group.bench_with_input(BenchmarkId::new("squares_compose", n), &n, |b, &n| {
            b.iter(|| {
                let g = pin!(squares_compose(n));
                consume_iter(g.into_iter());
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_generators);
criterion_main!(benches);
