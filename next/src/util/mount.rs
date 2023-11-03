use rustix::{
    fs::{mount as _mount, unmount as _unmount, MountFlags, UnmountFlags},
    io,
};

use crate::{fstab::MountInfo, Rc};

pub(crate) fn mount(mount_info: Rc<MountInfo>, flags: MountFlags) -> io::Result<()> {
    let MountInfo {
        fs_spec: source,
        mount_point: target,
        vfs_type,
        mount_options: data,
    } = mount_info.as_ref();
    _mount(
        source.as_ref(),
        target.as_ref(),
        vfs_type.as_ref(),
        flags,
        data.as_ref(),
    )
}

pub(crate) fn unmount(mount_info: Rc<MountInfo>, flags: UnmountFlags) -> io::Result<()> {
    let MountInfo { mount_point, .. } = mount_info.as_ref();
    _unmount(mount_point.as_ref(), flags)
}

#[derive(Debug, Clone)]
pub(crate) struct ProcMountInfoLine {
    // mount_id:
    // parent_id:
    // st_dev:
    // root:
    pub mount_point: Rc<str>,
    pub mount_options: Rc<str>,
    // optional_fields
    pub fs_type: Rc<str>,
    pub mount_source: Rc<str>,
    // super_option:
}

impl ProcMountInfoLine {
    pub(crate) fn parse(s: &str) -> Self {
        // let s: [&str; 4];
        let mut iter = s.trim().split_ascii_whitespace().skip(4);
        let mount_point = iter.next().unwrap().into();
        let mount_options = iter.next().unwrap().into();
        let mut iter = iter.skip_while(|&w| w == "-");
        let fs_type = iter.next().unwrap().into();
        let mount_source = iter.next().unwrap().into();
        Self {
            mount_point,
            mount_options,
            fs_type,
            mount_source,
        }
    }
}

pub(crate) fn mount_point_to_unit_name(name: &str) -> String {
    (if let Some(s) = name.strip_prefix('/') {
        if s.is_empty() {
            String::from('-')
        } else {
            s.replace('-', "\\x2d").replace('/', "-")
        }
    } else {
        name.replace('-', "\\x2d").replace('/', "-")
    } + ".mount")
        .into()
}
