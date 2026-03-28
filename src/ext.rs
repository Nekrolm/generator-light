use crate::core::pin::Pin;

use crate::{Generator, GeneratorState};

pub struct Iter<G>(G);

impl<G> Iterator for Iter<G>
where
    G: Generator<(), Return = ()> + Unpin,
{
    type Item = G::Yield;

    fn next(&mut self) -> Option<Self::Item> {
        let GeneratorState::Yield(val) = Pin::new(&mut self.0).resume(()) else {
            return None;
        };
        Some(val)
    }
}

pub fn from_iter<Item>(
    iter: impl IntoIterator<Item = Item>,
) -> impl Generator<Yield = Item, Return = ()> {
    struct GenIter<I: Iterator>(I);
    impl<I: Iterator> Generator for GenIter<I> {
        type Return = ();
        type Yield = I::Item;
        fn resume(self: Pin<&mut Self>, _value: ()) -> GeneratorState<Self::Yield, Self::Return> {
            // SAFETY: We are not moving out of the pinned field.
            match unsafe { self.get_unchecked_mut() }.0.next() {
                Some(val) => GeneratorState::Yield(val),
                None => GeneratorState::Complete(()),
            }
        }
    }
    GenIter(iter.into_iter())
}

pub const fn from_fn<Resume, Yield, Return>(
    f: impl FnMut(Resume) -> GeneratorState<Yield, Return>,
) -> impl Generator<Resume, Yield = Yield, Return = Return> {
    struct GenFn<F>(F);
    impl<F, R, Y, Out> Generator<R> for GenFn<F>
    where
        F: FnMut(R) -> GeneratorState<Y, Out>,
    {
        type Return = Out;
        type Yield = Y;
        fn resume(self: Pin<&mut Self>, value: R) -> GeneratorState<Self::Yield, Self::Return> {
            // SAFETY: We are not moving out of the pinned field.
            unsafe { self.get_unchecked_mut() }.0(value)
        }
    }
    GenFn(f)
}

pub trait GeneratorIterator {
    type Item;
    type Iter: Iterator<Item = Self::Item>;
    fn into_iter(self) -> Self::Iter;
}

impl<G> GeneratorIterator for G
where
    G: Generator<(), Return = ()> + Unpin,
{
    type Item = G::Yield;
    type Iter = Iter<G>;
    fn into_iter(self) -> Self::Iter {
        Iter(self)
    }
}

pub use either::Either;

pub trait GeneratorExt<R>: Generator<R> {
    fn receiving<F, U>(self, f: F) -> Receiving<Self, F>
    where
        Self: Sized,
        F: FnMut(U) -> R,
    {
        Receiving { g: self, f }
    }

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

    fn map_yield<U, F>(self, f: F) -> MapYield<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Yield) -> U,
    {
        MapYield { generator: self, f }
    }

    fn chain_with<G, F>(self, f: F) -> AndThen<Self, F, G>
    where
        Self: Sized,
        G: Generator<R, Yield = Self::Yield>,
        F: FnOnce(Self::Return) -> (G, R),
    {
        AndThen::Before {
            g: self,
            f: Some(f),
        }
    }

    fn join_with<C, F, I>(self, continue_inner: I, continue_outer: F) -> Flatten<Self, C, I, F>
    where
        Self: Sized,
        Self: Generator<R, Yield = C>,
        C: Generator<R>,
        I: FnMut() -> R,
        F: FnMut(C::Return) -> R,
    {
        Flatten {
            g: self,
            current: None,
            continue_outer,
            continue_inner,
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

pub struct MapYield<G, F> {
    generator: G,
    f: F,
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

impl<G, F, R, U> Generator<R> for MapYield<G, F>
where
    G: Generator<R>,
    F: FnMut(G::Yield) -> U,
{
    type Yield = U;
    type Return = G::Return;

    fn resume(self: Pin<&mut Self>, value: R) -> GeneratorState<Self::Yield, Self::Return> {
        // safety: we are not moving out from the reference
        let this = unsafe { self.get_unchecked_mut() };
        let g = unsafe { Pin::new_unchecked(&mut this.generator) };
        g.resume(value).map_yield(&mut this.f)
    }
}

pub enum AndThen<G1, F, G2> {
    Before { g: G1, f: Option<F> },
    After { g: G2 },
}

impl<G1, F, G2, R> Generator<R> for AndThen<G1, F, G2>
where
    G1: Generator<R>,
    F: FnOnce(G1::Return) -> (G2, R),
    G2: Generator<R, Yield = G1::Yield>,
{
    type Yield = G1::Yield;
    type Return = G2::Return;

    fn resume(mut self: Pin<&mut Self>, mut value: R) -> GeneratorState<Self::Yield, Self::Return> {
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        loop {
            match this {
                AndThen::Before { g, f } => match unsafe { Pin::new_unchecked(g) }.resume(value) {
                    GeneratorState::Complete(r) => {
                        let f = f.take().unwrap();
                        let (g2, y) = f(r);
                        *this = AndThen::After { g: g2 };
                        value = y;
                    }
                    GeneratorState::Yield(y) => return GeneratorState::Yield(y),
                },
                AndThen::After { g } => return unsafe { Pin::new_unchecked(g) }.resume(value),
            }
        }
    }
}

pub struct Flatten<G, C, I, F> {
    g: G,
    current: Option<C>,
    continue_inner: I,
    continue_outer: F,
}

impl<R, G, C, F, I> Generator<R> for Flatten<G, C, I, F>
where
    G: Generator<R, Yield = C>,
    C: Generator<R>,
    F: FnMut(C::Return) -> R,
    I: FnMut() -> R,
{
    type Yield = C::Yield;
    type Return = G::Return;

    fn resume(self: Pin<&mut Self>, mut value: R) -> GeneratorState<Self::Yield, Self::Return> {
        let this = unsafe { self.get_unchecked_mut() };
        loop {
            if let Some(current) = &mut this.current {
                match unsafe { Pin::new_unchecked(current) }.resume(value) {
                    GeneratorState::Complete(seed) => {
                        value = (this.continue_outer)(seed);
                        this.current = None;
                    }
                    GeneratorState::Yield(y) => return GeneratorState::Yield(y),
                }
            } else {
                match unsafe { Pin::new_unchecked(&mut this.g) }.resume(value) {
                    GeneratorState::Complete(r) => return GeneratorState::Complete(r),
                    GeneratorState::Yield(c) => {
                        this.current = Some(c);
                        value = (this.continue_inner)()
                    }
                }
            }
        }
    }
}

pub struct Once<Y>(Option<Y>);
pub struct OnceWith<F>(Option<F>);

impl<Y> Generator for Once<Y> {
    type Return = ();
    type Yield = Y;
    fn resume(self: Pin<&mut Self>, _value: ()) -> GeneratorState<Self::Yield, Self::Return> {
        let this = unsafe { self.get_unchecked_mut() };
        if let Some(y) = this.0.take() {
            GeneratorState::Yield(y)
        } else {
            GeneratorState::Complete(())
        }
    }
}

impl<F, R, Y> Generator<R> for OnceWith<F>
where
    F: FnOnce(R) -> Y,
{
    type Return = ();
    type Yield = Y;
    fn resume(self: Pin<&mut Self>, value: R) -> GeneratorState<Self::Yield, Self::Return> {
        let this = unsafe { self.get_unchecked_mut() };
        if let Some(f) = this.0.take() {
            GeneratorState::Yield(f(value))
        } else {
            GeneratorState::Complete(())
        }
    }
}

pub fn once<Y>(yielded: Y) -> impl Generator<Yield = Y, Return = ()> {
    Once(Some(yielded))
}
pub fn once_with<F, R, Y>(f: F) -> impl Generator<R, Yield = Y, Return = ()>
where
    F: FnOnce(R) -> Y,
{
    OnceWith(Some(f))
}

pub struct Receiving<G, F> {
    g: G,
    f: F,
}

impl<R1, R2, G, F> Generator<R1> for Receiving<G, F>
where
    F: FnMut(R1) -> R2,
    G: Generator<R2>,
{
    type Yield = G::Yield;
    type Return = G::Return;

    fn resume(self: Pin<&mut Self>, value: R1) -> GeneratorState<Self::Yield, Self::Return> {
        let this = unsafe { self.get_unchecked_mut() };
        let r2 = (this.f)(value);
        unsafe { Pin::new_unchecked(&mut this.g) }.resume(r2)
    }
}
