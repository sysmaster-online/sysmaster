use crate::error::Error;
use crate::escape::escape;
use crate::template::{unit_type, UnitType};
use nix::sys::utsname::UtsName;
use nix::unistd::{Uid, User};
use nix::{
    sys::utsname::uname,
    unistd::{Gid, Group},
};
use once_cell::sync::Lazy;
use os_release::OsRelease;
use std::{env, fs};

pub(crate) type SpecifierContext<'a> = (&'a str,); // (unit_name)

static OS_RELEASE: Lazy<OsRelease> =
    Lazy::new(|| OsRelease::new().expect("Failed to read os-release."));
static UTS_NAME: Lazy<UtsName> = Lazy::new(|| uname().expect("Failed to read system information."));
static BOOT_ID: Lazy<String> = Lazy::new(|| {
    fs::read_to_string("/proc/sys/kernel/random/boot_id").expect("Failed to read boot_id.")
});
static MACHINE_ID: Lazy<String> =
    Lazy::new(|| fs::read_to_string("/etc/machine-id").expect("Failed to read machine_id."));
static CURRENT_UID: Lazy<Uid> = Lazy::new(Uid::current);
static CURRENT_GID: Lazy<Gid> = Lazy::new(Gid::current);

// return Cow?
/// Resolves a specifier, writes directly to the end of buffer.
pub(crate) fn resolve(
    result: &mut String,
    specifier: char,
    context: SpecifierContext,
) -> Result<(), Error> {
    let in_system_mode = true;
    match specifier {
        'a' => {
            if let Some(res) = UTS_NAME.machine().to_str() {
                result.push_str(res);
            }
        }
        'A' => {
            if let Some(res) = OS_RELEASE.extra.get("IMAGE_VERSION") {
                result.push_str(res);
            }
        }
        'b' => result.push_str(&BOOT_ID),
        'B' => {
            if let Some(res) = OS_RELEASE.extra.get("BUILD_ID") {
                result.push_str(res);
            }
        }
        'C' => {
            if in_system_mode {
                result.push_str("/var/cache");
            } else if let Ok(res) = env::var("XDG_CACHE_HOME") {
                result.push_str(&res);
            } else {
                result.push_str("~/.cache");
            }
        }
        'd' => {
            if let Ok(res) = env::var("CREDENTIALS_DIRECTORY") {
                result.push_str(&res);
            }
        }
        'E' => {
            if in_system_mode {
                result.push_str("/etc");
            } else if let Ok(res) = env::var("XDG_CONFIG_HOME") {
                result.push_str(&res);
            } else {
                result.push_str("~/.config");
            }
        }
        'f' => result.push_str(context.0),
        'g' => {
            if in_system_mode {
                result.push_str("root");
            } else if let Some(gid) =
                Group::from_gid(*CURRENT_GID).expect("Failed to read current group info.")
            {
                result.push_str(&gid.name);
            }
        }
        'G' => {
            if in_system_mode {
                result.push('0');
            } else {
                result.push_str(&CURRENT_GID.to_string());
            }
        }
        'h' => {
            if in_system_mode {
                result.push_str("/root");
            } else if let Ok(res) = env::var("HOME") {
                result.push_str(&res);
            } else {
                result.push('~');
            }
        }
        'H' => {
            if let Some(res) = UTS_NAME.nodename().to_str() {
                result.push_str(res);
            }
        }
        'i' => {
            if let UnitType::Instance(instance_name, _) = unit_type(context.0)? {
                result.push_str(&escape(instance_name));
            }
        }
        'I' => {
            if let UnitType::Instance(instance_name, _) = unit_type(context.0)? {
                result.push_str(instance_name);
            }
        }
        'j' => {
            if let UnitType::Instance(instance_name, _) = unit_type(context.0)? {
                result.push_str(&escape(instance_name.split('-').last().unwrap()));
            } else {
                result.push_str(&escape(
                    context
                        .0
                        .split('.')
                        .next()
                        .unwrap()
                        .split('-')
                        .last()
                        .unwrap(),
                ));
            }
        }
        'J' => {
            if let UnitType::Instance(instance_name, _) = unit_type(context.0)? {
                result.push_str(instance_name.split('-').last().unwrap());
            } else {
                result.push_str(
                    context
                        .0
                        .split('.')
                        .next()
                        .unwrap()
                        .split('-')
                        .last()
                        .unwrap(),
                );
            }
        }
        'l' => result.push_str(
            UTS_NAME
                .nodename()
                .to_string_lossy()
                .split('.')
                .next()
                .unwrap(),
        ),
        'L' => {
            if in_system_mode {
                result.push_str("/var/log");
            } else if let Ok(res) = env::var("XDG_STATE_HOME") {
                result.push_str(&res);
                result.push_str("/log");
            } else {
                result.push_str("~/.local/state/log");
            }
        }
        'm' => result.push_str(&MACHINE_ID),
        'M' => {
            if let Some(res) = OS_RELEASE.extra.get("IMAGE_ID") {
                result.push_str(res)
            }
        }
        'n' => result.push_str(&escape(context.0)),
        'N' => result.push_str(&escape(context.0.split('.').next().unwrap())),
        'o' => result.push_str(&OS_RELEASE.id),
        'p' => {
            if let UnitType::Instance(instance_name, _) = unit_type(context.0)? {
                result.push_str(&escape(instance_name));
            } else {
                result.push_str(&escape(context.0.split('.').next().unwrap()));
            }
        }
        'P' => {
            if let UnitType::Instance(instance_name, _) = unit_type(context.0)? {
                result.push_str(instance_name);
            } else {
                result.push_str(context.0.split('.').next().unwrap());
            }
        }
        'q' => result.push_str(
            UTS_NAME
                .nodename()
                .to_string_lossy()
                .split('.')
                .next()
                .unwrap(),
        ),
        's' => {
            if let Ok(res) = env::var("SHELL") {
                result.push_str(&res);
            }
        }
        'S' => {
            if in_system_mode {
                result.push_str("/var/lib");
            } else if let Ok(res) = env::var("XDG_STATE_HOME") {
                result.push_str(&res);
            } else {
                result.push_str("~/.local/share");
            }
        }
        't' => {
            if in_system_mode {
                result.push_str("/run");
            } else if let Ok(res) = env::var("XDG_RUNTIME_DIR") {
                result.push_str(&res);
            } else {
                result.push_str("run/user/");
                result.push_str(&CURRENT_UID.to_string());
            }
        }
        'T' => {
            if let Ok(res) = env::var("TMPDIR") {
                result.push_str(&res);
            } else if let Ok(res) = env::var("TEMP") {
                result.push_str(&res);
            } else if let Ok(res) = env::var("TMP") {
                result.push_str(&res);
            } else {
                result.push_str("/tmp");
            }
        }
        'u' => {
            if let Some(res) = User::from_uid(*CURRENT_UID).expect("Failed to read user name.") {
                result.push_str(&res.name);
            }
        }
        'U' => result.push_str(&CURRENT_UID.to_string()),
        'v' => {
            if let Some(res) = UTS_NAME.release().to_str() {
                result.push_str(res);
            }
        }
        'V' => {
            if let Ok(res) = env::var("TMPDIR") {
                result.push_str(&res);
            } else if let Ok(res) = env::var("TEMP") {
                result.push_str(&res);
            } else if let Ok(res) = env::var("TMP") {
                result.push_str(&res);
            } else {
                result.push_str("/var/tmp");
            }
        }
        'w' => result.push_str(&OS_RELEASE.version_id),
        'W' => {
            if let Some(res) = OS_RELEASE.extra.get("VARIANT_ID") {
                result.push_str(res);
            }
        }
        // 'y' => {
        //     if let Some(res) = context.3.to_str() {
        //         result.push_str(res)
        //     }
        // }
        // 'Y' => {
        //     if let Some(res) = context.3.parent().expect("Invalid file path.").to_str() {
        //         result.push_str(res)
        //     }
        // }
        '%' => result.push('%'),
        _ => return Err(Error::InvalidSpecifierError { specifier }),
    };
    Ok(())
}
