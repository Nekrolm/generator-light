use crate::core::hint::unreachable_unchecked;
use crate::core::pin::Pin;
use crate::core::task::{Context, Poll};

use crate::GeneratorState;
use crate::{Yielder, fn_trait::FnOnceOutput};

use crate::gen_context::{GeneratorContext, make_yielder_waker};

enum Gen<Init, G> {
    Init(Option<Init>),
    Gen(G),
}

pub(crate) struct Generator<F, Y, R>
where
    F: for<'a> FnOnceOutput<Yielder<'a, Y, R>, R, Out: Future>,
{
    generator: Gen<F, <F as FnOnceOutput<Yielder<'static, Y, R>, R>>::Out>,
}

impl<F, Y, R, Output> Generator<F, Y, R>
where
    F: for<'a> FnOnceOutput<Yielder<'a, Y, R>, R, Out: Future<Output = Output>>,
{
    fn resume_impl(self: Pin<&mut Self>, val: R) -> GeneratorState<Y, Output> {
        // Safety: nor context, nor initialized future will be moved out
        // from this mutable reference
        let this = unsafe { self.get_unchecked_mut() };
        let gen_state: &mut Gen<_, _> = &mut this.generator;
        let state = GeneratorContext::new();
        let waker = unsafe { make_yielder_waker(&state) };
        let mut poll_context = Context::from_waker(&waker);

        let f = match gen_state {
            Gen::Gen(f) => f,
            Gen::Init(init) => {
                // in initial state it's always present
                let init = unsafe { init.take().unwrap_unchecked() };

                let f = init.call(Yielder::new(), val);
                *gen_state = Gen::Gen(f);
                // Poll one with initial seed
                let Gen::Gen(f) = gen_state else {
                    unsafe { unreachable_unchecked() }
                };
                let pinned = unsafe { Pin::new_unchecked(f) };
                return match pinned.poll(&mut poll_context) {
                    Poll::Ready(out) => GeneratorState::Complete(out),
                    Poll::Pending => state
                        .take_yielded()
                        .map_or(GeneratorState::Suspend, GeneratorState::Yield),
                };
            }
        };
        // self is pinned.
        // it won't be reinitialized again -> we can safely pin-project here
        let pinned = unsafe { Pin::new_unchecked(f) };
        state.resume(val);
        match pinned.poll(&mut poll_context) {
            Poll::Ready(out) => GeneratorState::Complete(out),
            Poll::Pending => state
                .take_yielded()
                .map_or(GeneratorState::Suspend, GeneratorState::Yield),
        }
    }

    pub(crate) const fn new(f: F) -> Self {
        Self {
            // ctx: GeneratorContext::new(),
            generator: Gen::Init(Some(f)),
            // _pinned: PhantomPinned,
        }
    }
}

impl<F, Y, R, Output> crate::Generator<R> for Generator<F, Y, R>
where
    F: for<'a> FnOnceOutput<Yielder<'a, Y, R>, R, Out: Future<Output = Output>>,
{
    type Yield = Y;
    type Return = Output;

    fn resume(self: Pin<&mut Self>, resume_val: R) -> GeneratorState<Self::Yield, Self::Return> {
        self.resume_impl(resume_val)
    }
}
