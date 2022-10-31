use std::fmt::Debug;
use std::mem;
use std::ops::Neg;
use std::ptr::null_mut;

///Auxiliary data structure added for signal processing
pub(crate) struct SigSet {
    sigset: libc::sigset_t,
}

impl SigSet {
    /// Initialize to include nothing.
    pub fn empty() -> SigSet {
        let mut sigset = mem::MaybeUninit::zeroed();
        let _ = unsafe { libc::sigemptyset(sigset.as_mut_ptr()) };

        unsafe {
            SigSet {
                sigset: sigset.assume_init(),
            }
        }
    }

    /// Add the specified signal to the set.
    pub fn add(&mut self, signal: libc::c_int) {
        unsafe {
            libc::sigaddset(
                &mut self.sigset as *mut libc::sigset_t,
                signal as libc::c_int,
            )
        };
    }

    pub fn pthread_sigmask(&self, oldset: &mut SigSet) {
        unsafe {
            libc::pthread_sigmask(libc::SIG_BLOCK, &self.sigset, &mut oldset.sigset);
        }
    }
}

impl Debug for SigSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SigSet")
            .field(&(&self.sigset as *const libc::sigset_t))
            .finish()
    }
}

#[derive(Debug)]
pub(crate) struct Signals {
    set: SigSet,
    oldset: SigSet,
    fd: libc::c_int,
}

impl Signals {
    pub fn new() -> Signals {
        Signals {
            set: SigSet::empty(),
            oldset: SigSet::empty(),
            fd: -1,
        }
    }

    pub fn fd(&self) -> libc::c_int {
        self.fd
    }

    pub fn reset_sigset(&mut self, signals: Vec<libc::c_int>) {
        unsafe {
            for sig in signals {
                self.set.add(sig);
            }
            self.set.pthread_sigmask(&mut self.oldset);
            self.fd = libc::signalfd(
                -1,
                &mut self.set.sigset as *const libc::sigset_t,
                libc::SFD_NONBLOCK, // SIG_BLOCK
            );
        }
    }

    pub fn restore_sigset(&mut self) {
        unsafe {
            libc::pthread_sigmask(libc::SIG_SETMASK, &self.oldset.sigset, null_mut());
            libc::close(self.fd as libc::c_int);
        }
    }

    pub fn read_signals(&mut self) -> std::io::Result<Option<libc::siginfo_t>> {
        let mut buffer = mem::MaybeUninit::<libc::siginfo_t>::zeroed();

        let size = mem::size_of_val(&buffer);
        let res = unsafe { libc::read(self.fd, buffer.as_mut_ptr() as *mut libc::c_void, size) };
        match res {
            x if x == size as isize => Ok(Some(unsafe { buffer.assume_init() })),
            x if x >= 0 => Ok(None),
            x => Err(std::io::Error::from_raw_os_error(x.neg() as i32)), //unreachable!("partial read on signalfd, x = {}", x),
        }
    }
}

impl Drop for Signals {
    fn drop(&mut self) {
        self.restore_sigset();
    }
}
