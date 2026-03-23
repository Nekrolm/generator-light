use core::{ops::DerefMut, pin::Pin};

use crate::{Generator, GeneratorState, Yielder, generator, yield_};

pub struct Iter<P>(Pin<P>);

impl<P> Iterator for Iter<P>
where
    P: DerefMut<Target: Generator<(), Return = ()>>,
{
    type Item = <P::Target as Generator<()>>::Yield;

    fn next(&mut self) -> Option<Self::Item> {
        let GeneratorState::Yield(val) = self.0.as_mut().resume(()) else {
            return None;
        };
        Some(val)
    }
}

pub fn from_iter<Item>(iter: impl IntoIterator<Item = Item>) -> impl Generator<Yield = Item> {
    generator(async move |mut yielder: Yielder<_, _>, _| {
        for x in iter {
            yield_!(yielder, x)
        }
    })
}

pub trait GeneratorIterator {
    type Item;
    type Iter: Iterator<Item = Self::Item>;
    fn into_iter(self) -> Self::Iter;
}

impl<P> GeneratorIterator for Pin<P>
where
    P: DerefMut<Target: Generator<(), Return = ()>>,
{
    type Item = <P::Target as Generator<()>>::Yield;
    type Iter = Iter<P>;
    fn into_iter(self) -> Self::Iter {
        Iter(self)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

pub trait GeneratorExt<R>: Generator<R> {
    fn compose<G: Generator<Self::Yield>>(self, then: G) -> Compose<Self, G>
    where
        Self: Sized,
    {
        Compose {
            first: self,
            second: then,
        }
    }

    fn map_complete<U, F>(self, f: F) -> MapComplete<Self, F>
    where
        Self: Sized,
        F: FnOnce(Self::Return) -> U,
    {
        MapComplete {
            generator: self,
            f: Some(f),
        }
    }
}

impl<G, R> GeneratorExt<R> for G where G: Generator<R> {}

pub struct Compose<A, B> {
    first: A,
    second: B,
}

impl<A, B, R> Generator<R> for Compose<A, B>
where
    A: Generator<R>,
    B: Generator<A::Yield>,
{
    type Return = Either<A::Return, B::Return>;
    type Yield = B::Yield;

    fn resume(mut self: Pin<&mut Self>, value: R) -> GeneratorState<Self::Yield, Self::Return> {
        let first = unsafe { self.as_mut().map_unchecked_mut(|this| &mut this.first) };
        let value = match first.resume(value) {
            GeneratorState::Complete(x) => return GeneratorState::Complete(Either::Left(x)),
            GeneratorState::Yield(value) => value,
        };
        let second = unsafe { self.map_unchecked_mut(|this| &mut this.second) };
        second.resume(value).map_complete(Either::Right)
    }
}

pub struct MapComplete<G, F> {
    generator: G,
    f: Option<F>,
}

impl<G, F, R, U> Generator<R> for MapComplete<G, F>
where
    G: Generator<R>,
    F: FnOnce(G::Return) -> U,
{
    type Yield = G::Yield;
    type Return = U;

    fn resume(mut self: Pin<&mut Self>, value: R) -> GeneratorState<Self::Yield, Self::Return> {
        let g = unsafe { self.as_mut().map_unchecked_mut(|this| &mut this.generator) };
        g.resume(value)
            .map_complete(|r| unsafe { self.get_unchecked_mut() }.f.take().unwrap()(r))
    }
}
