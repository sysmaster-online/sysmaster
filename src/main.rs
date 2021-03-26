use std::io::Error;

use signal_hook::consts::signal::SIGCHLD;
use signal_hook::iterator::SignalsInfo;
use signal_hook::iterator::exfiltrator::WithOrigin;

fn main() -> Result<(), Error> {
    let sigs = vec![SIGCHLD];
    
    let mut signals = SignalsInfo::<WithOrigin>::new(&sigs)?;

    for info in &mut signals {
        println!("Received a signal {:?}", info);
        #[cfg(feature="extended-siginfo")]
        match info.signal {
            SIGCHLD => {
                println!("CHLD received");
            }
        }
    }
    println!("Hello, world!");
    Ok(())
}
