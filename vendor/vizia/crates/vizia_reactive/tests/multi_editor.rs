//! One test process so global UI registration cannot interfere with unrelated tests.
use std::{cell::Cell, rc::Rc, sync::mpsc, thread};
use vizia_reactive::{Effect, Runtime, Scope, SignalGet, SignalUpdate, SyncSignal};

#[test]
fn editor_threads_route_updates_dispose_and_recreate() {
    enum Command {
        Drain(i32),
        Write(i32),
        WriteOther(SyncSignal<i32>, i32),
        Dispose,
    }
    fn editor() -> (
        mpsc::Sender<Command>,
        mpsc::Receiver<SyncSignal<i32>>,
        mpsc::Receiver<()>,
        thread::JoinHandle<()>,
    ) {
        let (commands, receiver) = mpsc::channel();
        let (signals, signal_receiver) = mpsc::channel();
        let (done, completed) = mpsc::channel();
        let handle = thread::spawn(move || {
            Runtime::init_on_ui_thread();
            // A second window on the same native thread must keep the registration alive.
            Runtime::init_on_ui_thread();
            Runtime::deinit_on_ui_thread();
            Runtime::assert_ui_thread();
            let scope = Scope::new();
            let observed = Rc::new(Cell::new(-1));
            let signal = scope.enter(|| {
                let signal = SyncSignal::new(0);
                let observed = observed.clone();
                Effect::new(move |_| observed.set(signal.get()));
                signal
            });
            signals.send(signal).unwrap();
            while let Ok(command) = receiver.recv() {
                match command {
                    Command::Drain(expected) => {
                        Runtime::drain_pending_work();
                        assert_eq!(observed.get(), expected);
                    }
                    Command::Write(value) => signal.set(value),
                    Command::WriteOther(other, value) => other.set(value),
                    Command::Dispose => {
                        scope.dispose();
                        assert!(
                            signal.try_get().is_none(),
                            "sync signal removed from global storage"
                        );
                        Runtime::deinit_on_ui_thread();
                        done.send(()).unwrap();
                        return;
                    }
                }
                done.send(()).unwrap();
            }
        });
        (commands, signal_receiver, completed, handle)
    }
    let (a, sa, da, ha) = editor();
    let signal_a = sa.recv().unwrap();
    let (b, sb, db, hb) = editor();
    let signal_b = sb.recv().unwrap();
    assert!(!Runtime::is_ui_thread(), "host thread is not registered");
    assert!(std::panic::catch_unwind(Runtime::assert_ui_thread).is_err());
    signal_a.set(11);
    signal_b.set(22);
    // B drains first; it must not steal A's pending update.
    b.send(Command::Drain(22)).unwrap();
    db.recv().unwrap();
    a.send(Command::Drain(11)).unwrap();
    da.recv().unwrap();
    a.send(Command::Write(33)).unwrap();
    da.recv().unwrap();
    a.send(Command::Drain(33)).unwrap();
    da.recv().unwrap();
    b.send(Command::Drain(22)).unwrap();
    db.recv().unwrap();
    // A registered UI thread must also route writes to another runtime's observers.
    a.send(Command::WriteOther(signal_b, 66)).unwrap();
    da.recv().unwrap();
    b.send(Command::Drain(66)).unwrap();
    db.recv().unwrap();
    a.send(Command::Dispose).unwrap();
    da.recv().unwrap();
    ha.join().unwrap();
    assert!(!Runtime::is_ui_thread(), "B still enforces UI ownership");
    let (c, sc, dc, hc) = editor();
    let signal_c = sc.recv().unwrap();
    signal_c.set(44);
    signal_b.set(55);
    c.send(Command::Drain(44)).unwrap();
    dc.recv().unwrap();
    b.send(Command::Drain(55)).unwrap();
    db.recv().unwrap();
    for (tx, rx, handle) in [(b, db, hb), (c, dc, hc)] {
        tx.send(Command::Dispose).unwrap();
        rx.recv().unwrap();
        handle.join().unwrap();
    }
}
