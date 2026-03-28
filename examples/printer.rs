#![allow(unused)]

use std::fmt::Display;
use std::iter::repeat_n;
use std::pin::pin;

use generator_light::Generator;
use generator_light::GeneratorState;
use generator_light::Yielder;
use generator_light::ext::GeneratorIterator;
use generator_light::ext::complete_with;
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
            from_iter(repeat_n('*', w))
                .map_complete(|_| '\n')
                .and_then(once)
        })
        .flatten()
}

// fn main() {
//     show_triangle(10).into_iter().for_each(|c| print!("{c}"));
// }

fn adjacent_difference() -> impl Generator<i32, Yield = i32, Return = Infallible> {
    complete_with(|x| x).and_then(|mut init| {
        from_fn(GeneratorState::Yield).receiving(move |x| x - std::mem::replace(&mut init, x))
    })
}

fn main() {
    from_iter((1..15).map(|x| x * x))
        .compose(adjacent_difference())
        .map_complete(drop)
        .into_iter()
        .for_each(|c| print!("{c}\n"));
}
