use std::marker::Send;

use tokio::signal::unix::{signal, SignalKind};

use crate::actor::Actors;

/// all the posix sig habdlers should be registered here
/// should be called under tokio rt
pub(crate) fn register_sig_handlers(actors: &Actors) {
    // handle ctrl-c/SIGINT
    register_signal_handler(SignalKind::interrupt(), || println!("SIGINT!"));
}

fn register_signal_handler<F>(signalkind: SignalKind, mut handler: F)
where
    F: FnMut() + Send + 'static,
{
    let mut sig = signal(signalkind).unwrap();
    tokio::spawn(async move {
        loop {
            sig.recv().await;
            handler();
        }
    });
}
