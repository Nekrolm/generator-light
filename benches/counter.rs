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
use generator_light::yield_;

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
    generator(async move |mut yilder: Yielder<_, _>, _| {
        for idx in 1..=n {
            let idx = black_box(idx);
            yield_!(yilder, idx * idx)
        }
    })
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

fn squares_manual_gen(n: usize) -> impl Generator<Yield = usize, Return = ()> {
    struct Gen {
        idx: usize,
        limit: usize,
    }

    impl Generator for Gen {
        type Return = ();
        type Yield = usize;
        fn resume(
            mut self: std::pin::Pin<&mut Self>,
            _value: (),
        ) -> GeneratorState<Self::Yield, Self::Return> {
            let cur = black_box(self.idx);
            // let cur = self.idx;
            self.idx += 1;
            if cur <= self.limit {
                GeneratorState::Yield(cur * cur)
            } else {
                GeneratorState::Complete(())
            }
        }
    }

    Gen { idx: 1, limit: n }
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

        group.bench_with_input(BenchmarkId::new("squares_compose", n), &n, |b, &n| {
            b.iter(|| {
                let g = pin!(squares_compose(n));
                consume_iter(g.into_iter());
            })
        });

        group.bench_with_input(BenchmarkId::new("manual_gen", n), &n, |b, &n| {
            b.iter(|| {
                let g = pin!(squares_manual_gen(n));
                consume_iter(g.into_iter());
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_generators);
criterion_main!(benches);
