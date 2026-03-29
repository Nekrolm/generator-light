use core::cell::Cell;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Poll, RawWaker, RawWakerVTable, Waker};

use core::ptr::NonNull;

enum YieldState<Y, R> {
    Resume(R),
    Yield(Option<Y>),
}

pub(crate) struct GeneratorContext<Y, R>(Cell<YieldState<Y, R>>);

pub struct Yielder<'a, Y, R> {
    state: PhantomData<(Y, R)>,
    brand: PhantomData<&'a mut ()>,
}

impl<Y, R> GeneratorContext<Y, R> {
    pub(crate) const fn new_resumed(value: R) -> Self {
        Self(Cell::new(YieldState::Resume(value)))
    }

    pub(crate) fn take_yielded(self) -> Option<Y> {
        let YieldState::Yield(val) = self.0.into_inner() else {
            unsafe { crate::core::hint::unreachable_unchecked() };
        };
        val
    }
}

unsafe fn get_context<Y, R>(ctx: &mut core::task::Context<'_>) -> NonNull<Cell<YieldState<Y, R>>> {
    unsafe { NonNull::new_unchecked(ctx.waker().data() as *mut ()) }.cast()
}

const YIELD_WAKER: RawWakerVTable = RawWakerVTable::new(
    |_| panic!("Clone is not allowed for yielder"),
    |_| {},
    |_| {},
    |_| {},
);

pub(crate) const unsafe fn make_yielder_waker<Y, R>(x: &GeneratorContext<Y, R>) -> Waker {
    unsafe { Waker::from_raw(RawWaker::new((&raw const x.0).cast(), &YIELD_WAKER)) }
}

#[repr(transparent)]
struct YieldFuture<'a, Y, R> {
    state: Option<Y>,
    _yileder: Yielder<'a, Y, R>,
}

impl<'a, Y, R> Future for YieldFuture<'a, Y, R> {
    type Output = R;

    #[inline(always)]
    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        // Safety:
        //    YieldFuture may be created only by Yielder. Yeilder exists only during generator
        //    execution. This means we are inside generator and cx has yilder_waker data inside
        let ctx = unsafe { get_context::<Y, R>(cx).as_ref() };
        if let Some(state) = unsafe { self.get_unchecked_mut() }.state.take() {
            // We have something to yield. Unconditionally return control to the context
            ctx.set(YieldState::Yield(Some(state)));
            return Poll::Pending;
        }

        match ctx.replace(YieldState::Yield(None)) {
            YieldState::Resume(r) => Poll::Ready(r),
            _ => Poll::Pending,
        }
    }
}

impl<Y, R> Yielder<'_, Y, R> {
    pub(crate) const fn new() -> Self {
        Yielder {
            state: PhantomData,
            brand: PhantomData,
        }
    }

    pub const fn yield_value<'a>(&'a mut self, value: Y) -> impl Future<Output = R> {
        YieldFuture {
            state: Some(value),
            _yileder: Yielder::<'a, Y, R>::new(),
        }
    }

    pub const fn suspend<'a>(&'a mut self) -> impl Future<Output = R> {
        YieldFuture {
            state: None,
            _yileder: Yielder::<'a, Y, R>::new(),
        }
    }
}

#[macro_export]
macro_rules! yield_ {
    ($yielder:ident, $value:expr) => {
        $yielder.yield_value($value).await
    };
}

#[macro_export]
macro_rules! suspend_ {
    ($yielder:ident) => {
        $yielder.suspend().await
    };
}
