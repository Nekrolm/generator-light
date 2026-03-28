use std::marker::PhantomData;

use crate::core::pin::Pin;
use crate::core::task::{Context, Poll};

use crate::GeneratorState;
use crate::{Yielder, fn_trait::FnOnceOutput};

use crate::gen_context::{GeneratorContext, make_yielder_waker};

pub(crate) struct FutGenerator<F, Y, R> {
    f: F,
    _gen_params: PhantomData<(Y, R)>,
}

impl<F, Y, R, Output> crate::Generator<R> for FutGenerator<F, Y, R>
where
    F: Future<Output = Output>,
{
    type Yield = Y;
    type Return = Output;

    #[inline(always)]
    fn resume(self: Pin<&mut Self>, value: R) -> GeneratorState<Self::Yield, Self::Return> {
        let f = unsafe { self.map_unchecked_mut(|this| &mut this.f) };
        let state = GeneratorContext::<Y, _>::new_resumed(value);
        let waker = unsafe { make_yielder_waker(&state) };
        let mut poll_context = Context::from_waker(&waker);
        match f.poll(&mut poll_context) {
            Poll::Pending => state
                .take_yielded()
                .map_or(GeneratorState::Suspend, GeneratorState::Yield),
            Poll::Ready(x) => GeneratorState::Complete(x),
        }
    }
}

pub(crate) fn fut_generator<F, Y, R, O>(f: F) -> impl crate::Generator<R, Yield = Y, Return = O>
where
    F: for<'a> FnOnceOutput<Yielder<'a, Y, R>, Out: Future<Output = O>>,
{
    FutGenerator {
        f: f.call(Yielder::<'static, Y, R>::new()),
        _gen_params: PhantomData,
    }
}
