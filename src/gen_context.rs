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

// pub struct Yielder<'a, Y, R> {
//     state: NonNull<Cell<YieldState<Y, R>>>,
//     brand: PhantomData<&'a mut ()>,
// }

pub struct Yielder<'a, Y, R> {
    state: PhantomData<(Y, R)>,
    brand: PhantomData<&'a mut ()>,
}

impl<Y, R> GeneratorContext<Y, R> {
    pub(crate) const fn new() -> Self {
        Self(Cell::new(YieldState::Yield(None)))
    }

    pub(crate) fn resume(&self, value: R) {
        self.0.set(YieldState::Resume(value));
    }

    pub(crate) fn take_yielded(&self) -> Y {
        let YieldState::Yield(Some(val)) = self.0.replace(YieldState::Yield(None)) else {
            // unsafe { unreachable_unchecked() }
            unreachable!("This should never happen if generator is built correctly");
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
        match unsafe {
            get_context(cx)
                .as_ref()
                .replace(YieldState::Yield(self.get_unchecked_mut().state.take()))
        } {
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
}

#[macro_export]
macro_rules! yield_ {
    ($yielder:ident, $value:expr) => {
        $yielder.yield_value($value).await
    };
}
