#![allow(unused)]

use std::fmt::Display;
use std::pin::pin;

use generator_light::Generator;
use generator_light::GeneratorState;
use generator_light::Yielder;
use generator_light::ext::GeneratorIterator;
use generator_light::ext::from_fn;
use generator_light::ext::from_iter;
use generator_light::ext::once;
use generator_light::ext::once_with;
use generator_light::generator;
use generator_light::yield_;

use generator_light::ext::GeneratorExt;

use std::convert::Infallible;

fn list_printer<D: Display>(
    sep: impl Display,
) -> impl Generator<D, Yield = (), Return = Infallible> {
    generator(async move |mut yielder: Yielder<_, _>, mut item: D| {
        print!("{item}");
        loop {
            item = yield_!(yielder, ());
            print!("{sep}{item}")
        }
    })
}

fn print_list(s: impl IntoIterator<Item: Display>, sep: impl Display) {
    let g = from_iter(s).compose(list_printer(sep)).map_complete(drop);
    let g = pin!(g);
    g.into_iter().for_each(drop);
}

fn show_triangle(h: usize) -> impl Generator<Yield = char, Return = ()> {
    from_iter(1..=h)
        .map_yield(|w| {
            from_iter(1..=w)
                .map_yield(|_| '*')
                .chain_with(|_| (once('\n'), ()))
        })
        .join_with(|| (), |_| ())
}

fn main() {
    show_triangle(10).into_iter().for_each(|c| print!("{c}"));
}
