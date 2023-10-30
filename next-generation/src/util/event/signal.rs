use futures::Future;
use tokio::signal::unix::{signal, Signal, SignalKind};

use crate::Actors;

/// all the posix sig habdlers should be registered here
/// should be called under tokio rt
pub(crate) fn register_sig_handlers(actors: &Actors) {
    // handle ctrl-c/SIGINT
    register_signal_handler(SignalKind::interrupt(), || println!("SIGINT!"));
}

fn register_signal_handler(signalkind: SignalKind, handler: impl FnMut()) {
    let sig = signal(signalkind).unwrap();
    tokio::spawn(async move {
        loop {
            signal.recv().await;
            handler();
        }
        t
    });
}
