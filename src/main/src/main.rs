use utils::syscall;

use std::io::Error;
use std::io::BufRead;
use std::fs::File;

use signal_hook::consts::signal::SIGCHLD;
use signal_hook::iterator::SignalsInfo;
use signal_hook::iterator::exfiltrator::WithOrigin;


#[derive(Debug)]
enum Proc1Error {
    IOError(std::io::Error),
    NoOption,
    ChildPidTooBig(u32, std::num::TryFromIntError),
}

impl std::convert::From<std::io::Error> for Proc1Error {
    fn from(e: std::io::Error) -> Self {
        Proc1Error::IOError(e)
    }
}

fn get_command() -> Result<(String, Vec<String>), Proc1Error> {
    let mut args = std::env::args();
    let _me = args.next();
    match args.next() {
        None => Err(Proc1Error::NoOption),
        Some(cmd) => Ok((cmd, args.collect())),
    }
}

fn parse_inittab() -> Result<(), Error> {
    let inittab_file = File::open("/etc/inittab")?;
    let buf_reader = std::io::BufReader::new(inittab_file);
    let lines = buf_reader.lines();

    // actual read out configuration from /etc/inittab
    for (num, line) in lines.enumerate() {
        let l = line.unwrap();
        if l.chars().nth(0).unwrap() == '#' {
            // skip
        } else {
            println!("{:?}", l);
        }
    }
    //FIXME: 
    return Ok(())
}


fn main() -> Result<(), Proc1Error> {
    match (get_command()) {
        Ok((cmd, args)) => execute_mode(cmd, args),
        Err(Proc1Error::NoOption) => standalone_mode(),
        Err(_) => Ok(()),
    };

    Ok(())
}

fn execute_mode(s: String, ar: Vec<String>) -> Result<(), Proc1Error> {
    println!("running to execute specific command");
    println!("{:?}", s);
    println!("{:?}", ar);
    return Ok(())
}

fn standalone_mode() -> Result<(), Proc1Error> {
    println!("running as standalone pid1");
    // parse system init configuration
    // inittab is obsolated by systemd, should we honor the legacy config in that ?
    // systemd use /etc/systemd/system/default.target, which is a sym link to actual target.
    // fork process to do system initializaion
    println!("PID: {:?}", syscall!(fork()));

    match parse_inittab() {
        Err(why) => panic!("{:?}", why),
        Ok(config) => println!("Using /etc/inittab"),
    };

    // reacting on signls
    // 1. collect SICHLD
    // 2. react on service failure (restart on event)
    // 3. handle system reboot
    let sigs = vec![SIGCHLD];
    
    let mut signals = SignalsInfo::<WithOrigin>::new(&sigs)?;

    for info in &mut signals {
        println!("Received a signal {:?}", info);
        #[cfg(feature = "extended-siginfo")]
        match info.signal {
            SIGCHLD => {
                println!("CHLD received");
            }
        }
    }
    return Ok(());
}
