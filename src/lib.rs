#![cfg_attr(not(feature = "std"), no_std)]

mod core {
    #[cfg(not(feature = "std"))]
    pub use core::*;
    #[cfg(feature = "std")]
    pub use std::*;
}

mod fut_generator;

pub(crate) mod fn_trait;
pub(crate) mod gen_context;

pub mod ext;
pub mod generator_state;

use core::{ops::DerefMut, pin::Pin};

use crate::fn_trait::FnOnceOutput;
pub use crate::gen_context::Yielder;
pub use crate::generator_state::GeneratorState;

pub trait Generator<R = ()> {
    type Yield;
    type Return;
    fn resume(self: Pin<&mut Self>, value: R) -> GeneratorState<Self::Yield, Self::Return>;
}

impl<R, G> Generator<R> for &mut G
where
    G: Generator<R> + ?Sized + Unpin,
{
    type Return = G::Return;
    type Yield = G::Yield;

    // #[inline]
    fn resume(mut self: Pin<&mut Self>, value: R) -> GeneratorState<Self::Yield, Self::Return> {
        G::resume(Pin::new(&mut **self), value)
    }
}

#[cfg(feature = "std")]
impl<R, G> Generator<R> for core::boxed::Box<G>
where
    G: Generator<R> + ?Sized + Unpin,
{
    type Return = G::Return;
    type Yield = G::Yield;

    // #[inline]
    fn resume(mut self: Pin<&mut Self>, value: R) -> GeneratorState<Self::Yield, Self::Return> {
        G::resume(Pin::new(&mut **self), value)
    }
}

impl<R, P> Generator<R> for Pin<P>
where
    P: DerefMut<Target: Generator<R>>,
{
    type Return = <P::Target as Generator<R>>::Return;
    type Yield = <P::Target as Generator<R>>::Yield;

    fn resume(self: Pin<&mut Self>, value: R) -> GeneratorState<Self::Yield, Self::Return> {
        <P::Target as Generator<R>>::resume(self.as_deref_mut(), value)
    }
}

/// To build generator, pass an async function with a parameter:
/// First parameter is Yielder -- context handle to yield generated item
pub fn generator<F, Resume, Yield, Return>(
    f: F,
) -> impl Generator<Resume, Yield = Yield, Return = Return>
where
    // F: AsyncFnOnce(Yielder<Yield, Resume>, Resume) -> Return,
    // but this has to be expressed in such ugly form
    for<'a> F: FnOnceOutput<Yielder<'a, Yield, Resume>, Out: Future<Output = Return>>,
{
    fut_generator::fut_generator(f)
}

#[cfg(test)]
mod tests {

    use crate::core::pin::pin;

    use crate::{ext::GeneratorIterator, *};

    fn squares(n: usize) -> impl Generator<Yield = usize, Return = ()> {
        generator(async move |mut this: Yielder<_, _>| {
            for x in 1..=n {
                yield_!(this, x * x);
            }
        })
    }

    fn tok_generator<'a>(s: &'a str) -> impl Generator<Yield = &'a str, Return = ()> {
        generator(async move |mut this: Yielder<_, _>| {
            for tok in s.split_whitespace() {
                yield_!(this, tok);
            }
        })
    }

    #[test]
    fn test_squares() {
        let g = pin!(squares(5));
        let mut g = g.into_iter();
        assert_eq!(g.next(), Some(1));
        assert_eq!(g.next(), Some(4));
        assert_eq!(g.next(), Some(9));
        assert_eq!(g.next(), Some(16));
        assert_eq!(g.next(), Some(25));
        assert_eq!(g.next(), None);
    }

    #[test]
    fn test_tokens() {
        let g = pin!(tok_generator("hello world"));
        let mut g = g.into_iter();
        assert_eq!(g.next(), Some("hello"));
        assert_eq!(g.next(), Some("world"));
        assert_eq!(g.next(), None);
    }
}
