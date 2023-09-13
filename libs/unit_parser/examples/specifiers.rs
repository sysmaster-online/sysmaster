#![allow(non_snake_case)]

use unit_parser::prelude::*;

#[derive(UnitConfig, Debug)]
#[unit(suffix = "unit")]
struct Unit {
    #[section(must)]
    Section: Section,
}

#[derive(UnitSection, Debug)]
struct Section {
    #[entry(must)]
    Arch: String,
    #[entry(must)]
    OSImageVersion: String,
    #[entry(must)]
    BootID: String,
    #[entry(must)]
    OSBuildID: String,
    #[entry(must)]
    CacheRoot: String,
    #[entry(must)]
    CredentialsDir: String,
    #[entry(must)]
    ConfigRoot: String,
    #[entry(must)]
    UnescapedFilename: String,
    #[entry(must)]
    UserGroup: String,
    #[entry(must)]
    UserGID: String,
    #[entry(must)]
    UserHomeDir: String,
    #[entry(must)]
    HostName: String,
    #[entry(must)]
    InstanceName: String,
    #[entry(must)]
    UnescapedInstanceName: String,
    #[entry(must)]
    FinalComponentOfThePrefix: String,
    #[entry(must)]
    UnescapedFinalComponentOfThePrefix: String,
    #[entry(must)]
    ShortHostName: String,
    #[entry(must)]
    LogDirRoot: String,
    #[entry(must)]
    MachineID: String,
    #[entry(must)]
    OSImageID: String,
    #[entry(must)]
    FullUnitName: String,
    #[entry(must)]
    FullUnitNameWithoutSuffix: String,
    #[entry(must)]
    OSID: String,
    #[entry(must)]
    PrefixName: String,
    #[entry(must)]
    UnescapedPrefixName: String,
    #[entry(must)]
    PrettyHostName: String,
    #[entry(must)]
    UserShell: String,
    #[entry(must)]
    StateDirRoot: String,
    #[entry(must)]
    RuntimeDirRoot: String,
    #[entry(must)]
    TempDirRoot: String,
    #[entry(must)]
    UserName: String,
    #[entry(must)]
    UserUID: String,
    #[entry(must)]
    KernelRelease: String,
    #[entry(must)]
    PersistTempDirRoot: String,
    #[entry(must)]
    OSVersionID: String,
    #[entry(must)]
    OSVariantID: String,
    #[entry(must)]
    FragmentPath: String,
    #[entry(must)]
    FragmentDir: String,
}

fn main() {
    let user_result =
        Unit::load_named(vec!["libs/unit_parser/examples"], "specifiers", false).unwrap();
    println!("result in user mode: {:#?}", user_result);

    let root_result =
        Unit::load_named(vec!["libs/unit_parser/examples"], "specifiers", false).unwrap();
    println!("result in root mode: {:#?}", root_result);
}
