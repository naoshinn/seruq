use std::{cell::RefCell, error::Error, fmt, mem, rc::Rc};

use seruq_core::{
    Color, CreateSurfaceError, Node, PhysicalSize, PlatformCommand, PlatformEvent, PlatformHandler,
    Point, PresentableFrame, Rect, RectNode, RenderPlan, Renderer, Size, Surface, SurfaceTarget,
    Ucr, project_ucr,
};
use seruq_runtime::{DefaultRuntime, DefaultRuntimeError};

#[derive(Debug)]
struct FakeError;

impl fmt::Display for FakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("fake error")
    }
}

impl Error for FakeError {}

#[derive(Debug, Clone, PartialEq)]
enum Call {
    CreateSurface(PhysicalSize),
    Resize(PhysicalSize),
    Acquire,
    Render(RenderPlan),
    Present,
    DropSurface,
}

#[derive(Default)]
struct Log {
    calls: Vec<Call>,
    frame_unavailable: bool,
    fail_next: Option<FailurePoint>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FailurePoint {
    CreateSurface,
    Resize,
    Acquire,
    Render,
    Present,
}

impl Log {
    fn should_fail(&mut self, point: FailurePoint) -> bool {
        if self.fail_next == Some(point) {
            self.fail_next = None;
            true
        } else {
            false
        }
    }
}

type SharedLog = Rc<RefCell<Log>>;

struct FakeSurfaceTarget {
    _live: Rc<()>,
}

impl SurfaceTarget for FakeSurfaceTarget {}

struct FakeFrame {
    log: SharedLog,
}

impl PresentableFrame for FakeFrame {
    type Error = FakeError;

    fn present(self) -> Result<(), Self::Error> {
        let mut log = self.log.borrow_mut();
        log.calls.push(Call::Present);
        if log.should_fail(FailurePoint::Present) {
            Err(FakeError)
        } else {
            Ok(())
        }
    }
}

struct FakeSurface {
    log: SharedLog,
}

impl Surface for FakeSurface {
    type Error = FakeError;
    type Frame<'a> = FakeFrame;

    fn resize(&mut self, size: PhysicalSize) -> Result<(), Self::Error> {
        let mut log = self.log.borrow_mut();
        log.calls.push(Call::Resize(size));
        if log.should_fail(FailurePoint::Resize) {
            Err(FakeError)
        } else {
            Ok(())
        }
    }

    fn acquire(&mut self) -> Result<Option<Self::Frame<'_>>, Self::Error> {
        let mut log = self.log.borrow_mut();
        log.calls.push(Call::Acquire);

        if log.should_fail(FailurePoint::Acquire) {
            return Err(FakeError);
        }

        if log.frame_unavailable {
            return Ok(None);
        }

        Ok(Some(FakeFrame {
            log: Rc::clone(&self.log),
        }))
    }
}

impl Drop for FakeSurface {
    fn drop(&mut self) {
        self.log.borrow_mut().calls.push(Call::DropSurface);
    }
}

struct FakeRenderer {
    log: SharedLog,
}

impl Renderer<FakeSurfaceTarget> for FakeRenderer {
    type Surface = FakeSurface;
    type Error = FakeError;

    fn create_surface(
        &mut self,
        target: FakeSurfaceTarget,
        size: PhysicalSize,
    ) -> Result<Self::Surface, CreateSurfaceError<FakeSurfaceTarget, Self::Error>> {
        let mut log = self.log.borrow_mut();
        log.calls.push(Call::CreateSurface(size));
        if log.should_fail(FailurePoint::CreateSurface) {
            return Err(CreateSurfaceError::new(target, FakeError));
        }
        drop(log);
        Ok(FakeSurface {
            log: Rc::clone(&self.log),
        })
    }

    fn render(
        &mut self,
        _frame: &mut <Self::Surface as Surface>::Frame<'_>,
        plan: &RenderPlan,
    ) -> Result<(), Self::Error> {
        let mut log = self.log.borrow_mut();
        log.calls.push(Call::Render(plan.clone()));
        if log.should_fail(FailurePoint::Render) {
            Err(FakeError)
        } else {
            Ok(())
        }
    }
}

type Event = PlatformEvent<FakeSurfaceTarget>;

struct Harness {
    runtime: DefaultRuntime<FakeSurfaceTarget, FakeRenderer>,
    ucr: Ucr,
    log: SharedLog,
    live_targets: Rc<()>,
}

impl Harness {
    fn new() -> Self {
        let ucr = Ucr::new(Node::Rect(RectNode::new(
            Rect::new(
                Point::new(100.0, 200.0).unwrap(),
                Size::new(300.0, 400.0).unwrap(),
            ),
            Color::new(1.0, 1.0, 1.0).unwrap(),
        )));
        let log = SharedLog::default();
        let renderer = FakeRenderer {
            log: Rc::clone(&log),
        };

        Self {
            runtime: DefaultRuntime::new(ucr.clone(), renderer),
            ucr,
            log,
            live_targets: Rc::new(()),
        }
    }

    fn send(&mut self, event: Event) -> Vec<PlatformCommand> {
        self.send_result(event).unwrap()
    }

    fn send_result(
        &mut self,
        event: Event,
    ) -> Result<Vec<PlatformCommand>, DefaultRuntimeError<FakeError, FakeError>> {
        self.runtime.handle_event(event)
    }

    fn take_calls(&self) -> Vec<Call> {
        mem::take(&mut self.log.borrow_mut().calls)
    }

    fn window_created(&self, size: PhysicalSize) -> Event {
        PlatformEvent::WindowCreated {
            target: FakeSurfaceTarget {
                _live: Rc::clone(&self.live_targets),
            },
            size,
        }
    }

    fn held_targets(&self) -> usize {
        Rc::strong_count(&self.live_targets) - 1
    }

    fn make_frame_unavailable(&self) {
        self.log.borrow_mut().frame_unavailable = true;
    }

    fn fail_next(&self, point: FailurePoint) {
        self.log.borrow_mut().fail_next = Some(point);
    }

    fn expected_plan(&self) -> RenderPlan {
        project_ucr(&self.ucr)
    }
}

fn window_size() -> PhysicalSize {
    PhysicalSize::new(500, 600)
}

fn other_size() -> PhysicalSize {
    PhysicalSize::new(700, 800)
}

fn zero_size() -> PhysicalSize {
    PhysicalSize::new(0, 0)
}

fn resized(size: PhysicalSize) -> Event {
    PlatformEvent::Resized { size }
}

fn after(
    setup: fn() -> Harness,
    event: impl FnOnce(&Harness) -> Event,
    expected_commands: Vec<PlatformCommand>,
    expected_calls: Vec<Call>,
) -> Harness {
    let mut harness = setup();
    let event = event(&harness);
    assert_eq!(harness.send(event), expected_commands, "fixture commands");
    assert_eq!(harness.take_calls(), expected_calls, "fixture calls");
    harness
}

fn idle() -> Harness {
    Harness::new()
}

fn awaiting_window() -> Harness {
    after(
        idle,
        |_| PlatformEvent::Resumed,
        vec![PlatformCommand::CreateWindow],
        vec![],
    )
}

fn awaiting_surface() -> Harness {
    after(
        awaiting_window,
        |h| h.window_created(zero_size()),
        vec![],
        vec![],
    )
}

fn ready() -> Harness {
    after(
        awaiting_window,
        |h| h.window_created(window_size()),
        vec![PlatformCommand::RequestRedraw],
        vec![Call::CreateSurface(window_size())],
    )
}

fn minimized() -> Harness {
    after(ready, |_| resized(zero_size()), vec![], vec![])
}

fn suspended() -> Harness {
    after(
        ready,
        |_| PlatformEvent::Suspended,
        vec![],
        vec![Call::DropSurface],
    )
}

fn suspended_while_awaiting_window() -> Harness {
    after(
        awaiting_window,
        |_| PlatformEvent::Suspended,
        vec![],
        vec![],
    )
}

fn suspended_while_awaiting_surface() -> Harness {
    after(
        awaiting_surface,
        |_| PlatformEvent::Suspended,
        vec![],
        vec![],
    )
}

fn finished() -> Harness {
    after(
        ready,
        |_| PlatformEvent::CloseRequested,
        vec![PlatformCommand::Exit],
        vec![Call::DropSurface],
    )
}

fn probe(harness: &Harness) -> Vec<Event> {
    vec![
        PlatformEvent::RedrawRequested,
        resized(other_size()),
        PlatformEvent::Resumed,
        harness.window_created(window_size()),
    ]
}

type IgnoreCase = (&'static str, fn() -> Harness, fn(&Harness) -> Event);

fn assert_ignores(case: &str, setup: fn() -> Harness, event: fn(&Harness) -> Event) {
    let mut subject = setup();
    let mut reference = setup();

    let event = event(&subject);
    assert_eq!(subject.send(event), vec![], "{case}: returned commands");
    assert_eq!(
        subject.take_calls(),
        vec![],
        "{case}: touched renderer or surface"
    );

    let subject_probe = probe(&subject);
    let reference_probe = probe(&reference);
    for (step, (subject_event, reference_event)) in
        subject_probe.into_iter().zip(reference_probe).enumerate()
    {
        assert_eq!(
            subject.send(subject_event),
            reference.send(reference_event),
            "{case}: commands differ at probe step {step}"
        );
        assert_eq!(
            subject.take_calls(),
            reference.take_calls(),
            "{case}: calls differ at probe step {step}"
        );
    }
}

#[test]
fn ignored_events_leave_runtime_unchanged() {
    let cases: &[IgnoreCase] = &[
        ("awaiting window: resumed", awaiting_window, |_| {
            PlatformEvent::Resumed
        }),
        ("idle: window created", idle, |h| {
            h.window_created(window_size())
        }),
        ("ready: window created", ready, |h| {
            h.window_created(window_size())
        }),
        ("awaiting surface: resumed", awaiting_surface, |_| {
            PlatformEvent::Resumed
        }),
        ("awaiting surface: window created", awaiting_surface, |h| {
            h.window_created(window_size())
        }),
        ("awaiting surface: zero resize", awaiting_surface, |_| {
            resized(zero_size())
        }),
        ("ready: resumed", ready, |_| PlatformEvent::Resumed),
        ("idle: redraw", idle, |_| PlatformEvent::RedrawRequested),
        ("awaiting window: redraw", awaiting_window, |_| {
            PlatformEvent::RedrawRequested
        }),
        ("awaiting surface: redraw", awaiting_surface, |_| {
            PlatformEvent::RedrawRequested
        }),
        ("ready: same size resize", ready, |_| resized(window_size())),
        ("minimized: redraw", minimized, |_| {
            PlatformEvent::RedrawRequested
        }),
        ("idle: resize", idle, |_| resized(window_size())),
        ("awaiting window: resize", awaiting_window, |_| {
            resized(window_size())
        }),
        ("suspended: redraw", suspended, |_| {
            PlatformEvent::RedrawRequested
        }),
        (
            "suspended awaiting window: window created",
            suspended_while_awaiting_window,
            |h| h.window_created(window_size()),
        ),
        ("idle: suspended", idle, |_| PlatformEvent::Suspended),
        ("finished: close", finished, |_| {
            PlatformEvent::CloseRequested
        }),
        ("finished: resumed", finished, |_| PlatformEvent::Resumed),
        ("finished: window created", finished, |h| {
            h.window_created(window_size())
        }),
        ("finished: resize", finished, |_| resized(other_size())),
        ("finished: redraw", finished, |_| {
            PlatformEvent::RedrawRequested
        }),
        ("finished: suspended", finished, |_| {
            PlatformEvent::Suspended
        }),
    ];

    for &(name, setup, event) in cases {
        assert_ignores(name, setup, event);
    }
}

#[test]
fn idle_receiving_resumed_returns_create_window() {
    let mut runtime = idle();

    assert_eq!(
        runtime.send(PlatformEvent::Resumed),
        vec![PlatformCommand::CreateWindow]
    );
    assert_eq!(runtime.take_calls(), vec![]);
}

#[test]
fn awaiting_window_receiving_window_created_creates_surface_and_requests_redraw() {
    let mut runtime = awaiting_window();
    let event = runtime.window_created(window_size());

    assert_eq!(runtime.send(event), vec![PlatformCommand::RequestRedraw]);
    assert_eq!(
        runtime.take_calls(),
        vec![Call::CreateSurface(window_size())]
    );
}

#[test]
fn awaiting_window_receiving_zero_sized_window_created_defers_surface_creation() {
    let mut runtime = awaiting_window();
    let event = runtime.window_created(zero_size());

    assert_eq!(runtime.send(event), vec![]);
    assert_eq!(runtime.take_calls(), vec![]);
}

#[test]
fn awaiting_surface_receiving_resized_creates_surface_and_requests_redraw() {
    let mut runtime = awaiting_surface();

    assert_eq!(
        runtime.send(resized(window_size())),
        vec![PlatformCommand::RequestRedraw]
    );
    assert_eq!(
        runtime.take_calls(),
        vec![Call::CreateSurface(window_size())]
    );
}

#[test]
fn ready_receiving_redraw_requested_renders_and_presents() {
    let mut runtime = ready();
    let plan = runtime.expected_plan();

    assert_eq!(runtime.send(PlatformEvent::RedrawRequested), vec![]);
    assert_eq!(
        runtime.take_calls(),
        vec![Call::Acquire, Call::Render(plan), Call::Present]
    );
}

#[test]
fn ready_receiving_redraw_requested_skips_rendering_when_no_frame_is_available() {
    let mut runtime = ready();
    runtime.make_frame_unavailable();

    assert_eq!(runtime.send(PlatformEvent::RedrawRequested), vec![]);
    assert_eq!(runtime.take_calls(), vec![Call::Acquire]);
}

#[test]
fn ready_receiving_resized_resizes_surface_and_requests_redraw() {
    let mut runtime = ready();

    assert_eq!(
        runtime.send(resized(other_size())),
        vec![PlatformCommand::RequestRedraw]
    );
    assert_eq!(runtime.take_calls(), vec![Call::Resize(other_size())]);
}

#[test]
fn ready_receiving_zero_sized_resized_does_not_resize_surface() {
    let mut runtime = ready();

    assert_eq!(runtime.send(resized(zero_size())), vec![]);
    assert_eq!(runtime.take_calls(), vec![]);
}

#[test]
fn minimized_receiving_previous_size_resized_resizes_and_requests_redraw() {
    let mut runtime = minimized();

    assert_eq!(
        runtime.send(resized(window_size())),
        vec![PlatformCommand::RequestRedraw]
    );
    assert_eq!(runtime.take_calls(), vec![Call::Resize(window_size())]);
}

#[test]
fn minimized_receiving_resized_resizes_surface_and_requests_redraw() {
    let mut runtime = minimized();

    assert_eq!(
        runtime.send(resized(other_size())),
        vec![PlatformCommand::RequestRedraw]
    );
    assert_eq!(runtime.take_calls(), vec![Call::Resize(other_size())]);
}

#[test]
fn ready_receiving_suspended_drops_surface() {
    let mut runtime = ready();

    assert_eq!(runtime.send(PlatformEvent::Suspended), vec![]);
    assert_eq!(runtime.take_calls(), vec![Call::DropSurface]);
}

#[test]
fn suspended_receiving_resumed_returns_create_window() {
    let mut runtime = suspended();

    assert_eq!(
        runtime.send(PlatformEvent::Resumed),
        vec![PlatformCommand::CreateWindow]
    );
    assert_eq!(runtime.take_calls(), vec![]);
}

#[test]
fn suspended_while_awaiting_window_receiving_resumed_returns_create_window() {
    let mut runtime = suspended_while_awaiting_window();

    assert_eq!(
        runtime.send(PlatformEvent::Resumed),
        vec![PlatformCommand::CreateWindow]
    );
    assert_eq!(runtime.take_calls(), vec![]);
}

#[test]
fn suspended_while_awaiting_surface_receiving_resumed_returns_create_window() {
    let mut runtime = suspended_while_awaiting_surface();

    assert_eq!(
        runtime.send(PlatformEvent::Resumed),
        vec![PlatformCommand::CreateWindow]
    );
    assert_eq!(runtime.take_calls(), vec![]);
    assert_eq!(runtime.held_targets(), 0);
}

#[test]
fn idle_receiving_close_requested_returns_exit() {
    let mut runtime = idle();

    assert_eq!(
        runtime.send(PlatformEvent::CloseRequested),
        vec![PlatformCommand::Exit]
    );
    assert_eq!(runtime.take_calls(), vec![]);
}

#[test]
fn awaiting_window_receiving_close_requested_returns_exit() {
    let mut runtime = awaiting_window();

    assert_eq!(
        runtime.send(PlatformEvent::CloseRequested),
        vec![PlatformCommand::Exit]
    );
    assert_eq!(runtime.take_calls(), vec![]);
}

#[test]
fn awaiting_surface_receiving_close_requested_returns_exit() {
    let mut runtime = awaiting_surface();

    assert_eq!(
        runtime.send(PlatformEvent::CloseRequested),
        vec![PlatformCommand::Exit]
    );
    assert_eq!(runtime.take_calls(), vec![]);
}

#[test]
fn ready_receiving_close_requested_drops_surface_and_returns_exit() {
    let mut runtime = ready();

    assert_eq!(
        runtime.send(PlatformEvent::CloseRequested),
        vec![PlatformCommand::Exit]
    );
    assert_eq!(runtime.take_calls(), vec![Call::DropSurface]);
}

#[test]
fn ready_receiving_shutdown_drops_surface_and_ignores_later_events() {
    let mut runtime = ready();

    runtime.runtime.shutdown();
    assert_eq!(runtime.take_calls(), vec![Call::DropSurface]);

    for (step, event) in probe(&runtime).into_iter().enumerate() {
        assert_eq!(runtime.send(event), vec![], "commands at probe step {step}");
        assert_eq!(runtime.take_calls(), vec![], "calls at probe step {step}");
    }
}

#[test]
fn failed_surface_creation_retains_target_for_retry() {
    let mut runtime = awaiting_window();
    runtime.fail_next(FailurePoint::CreateSurface);
    let event = runtime.window_created(window_size());

    assert!(matches!(
        runtime.send_result(event),
        Err(DefaultRuntimeError::CreateSurface(_))
    ));
    assert_eq!(
        runtime.take_calls(),
        vec![Call::CreateSurface(window_size())]
    );
    assert_eq!(runtime.held_targets(), 1);

    assert_eq!(
        runtime.send(resized(other_size())),
        vec![PlatformCommand::RequestRedraw]
    );
    assert_eq!(
        runtime.take_calls(),
        vec![Call::CreateSurface(other_size())]
    );
    assert_eq!(runtime.held_targets(), 0);
}

#[test]
fn failed_resize_keeps_previous_size_and_can_retry() {
    let mut runtime = ready();
    runtime.fail_next(FailurePoint::Resize);

    assert!(matches!(
        runtime.send_result(resized(other_size())),
        Err(DefaultRuntimeError::Resize(_))
    ));
    assert_eq!(runtime.take_calls(), vec![Call::Resize(other_size())]);

    assert_eq!(
        runtime.send(resized(other_size())),
        vec![PlatformCommand::RequestRedraw]
    );
    assert_eq!(runtime.take_calls(), vec![Call::Resize(other_size())]);
}

#[test]
fn failed_acquire_does_not_render_and_can_retry() {
    let mut runtime = ready();
    runtime.fail_next(FailurePoint::Acquire);

    assert!(matches!(
        runtime.send_result(PlatformEvent::RedrawRequested),
        Err(DefaultRuntimeError::Acquire(_))
    ));
    assert_eq!(runtime.take_calls(), vec![Call::Acquire]);

    assert_eq!(runtime.send(PlatformEvent::RedrawRequested), vec![]);
    assert_eq!(
        runtime.take_calls(),
        vec![
            Call::Acquire,
            Call::Render(runtime.expected_plan()),
            Call::Present
        ]
    );
}

#[test]
fn failed_render_does_not_present_and_can_retry() {
    let mut runtime = ready();
    let plan = runtime.expected_plan();
    runtime.fail_next(FailurePoint::Render);

    assert!(matches!(
        runtime.send_result(PlatformEvent::RedrawRequested),
        Err(DefaultRuntimeError::Render(_))
    ));
    assert_eq!(
        runtime.take_calls(),
        vec![Call::Acquire, Call::Render(plan.clone())]
    );

    assert_eq!(runtime.send(PlatformEvent::RedrawRequested), vec![]);
    assert_eq!(
        runtime.take_calls(),
        vec![Call::Acquire, Call::Render(plan), Call::Present]
    );
}

#[test]
fn failed_present_can_retry_redraw() {
    let mut runtime = ready();
    let plan = runtime.expected_plan();
    runtime.fail_next(FailurePoint::Present);

    assert!(matches!(
        runtime.send_result(PlatformEvent::RedrawRequested),
        Err(DefaultRuntimeError::Present(_))
    ));
    assert_eq!(
        runtime.take_calls(),
        vec![Call::Acquire, Call::Render(plan.clone()), Call::Present]
    );

    assert_eq!(runtime.send(PlatformEvent::RedrawRequested), vec![]);
    assert_eq!(
        runtime.take_calls(),
        vec![Call::Acquire, Call::Render(plan), Call::Present]
    );
}

#[test]
fn shutdown_while_awaiting_surface_releases_target() {
    let mut runtime = awaiting_surface();
    assert_eq!(runtime.held_targets(), 1);

    runtime.runtime.shutdown();
    assert_eq!(runtime.held_targets(), 0);
    assert_eq!(runtime.take_calls(), vec![]);
}
