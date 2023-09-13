#![allow(non_snake_case, dead_code)]

use chrono::Duration;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use unit_parser::prelude::*;

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("load example", |b| {
        b.iter(|| {
            ExampleUnit::load_named(
                black_box(vec!["../lib/examples"]),
                black_box("example"),
                false,
            )
            .unwrap();
        })
    })
    .bench_function("load specifiers", |b| {
        b.iter(|| {
            SpecifierUnit::load_named(
                black_box(vec!["../lib/examples"]),
                black_box("specifiers"),
                false,
            )
            .unwrap();
            SpecifierUnit::load_named(
                black_box(vec!["../lib/examples"]),
                black_box("specifiers"),
                false,
            )
            .unwrap();
        })
    })
    .bench_function("load subdir", |b| {
        b.iter(|| {
            SubdirUnit::load_named(
                black_box(vec!["../lib/examples/subdir"]),
                black_box("subdir.unit"),
                false,
            )
            .unwrap();
        })
    });
}

#[derive(UnitConfig, Debug)]
#[unit(suffix = "unit")]
pub struct ExampleUnit {
    #[section(default, key = "AlternativeKey")]
    pub Section1: SimpleSection,

    #[section(must)]
    pub Section2: AdvancedSection,

    pub Section3: Option<OptionalSection>,
}

#[derive(UnitSection, Debug)]
pub struct SimpleSection {
    #[entry(must)]
    pub Field: String,
}

impl Default for SimpleSection {
    fn default() -> Self {
        Self {
            Field: "value".to_string(),
        }
    }
}

#[derive(UnitSection, Debug)]
pub struct AdvancedSection {
    #[entry(must)]
    pub Regular: String,

    #[entry(must)]
    Private: String,

    #[entry(must)]
    Enum: MyEnum,

    #[entry(key = "AlternativeKey", must)]
    CustomNamed: String,

    #[entry(default = "default-value")]
    DefaultValued: String,

    #[entry(must)]
    Duration: Duration,

    #[entry(multiple)]
    Multiple: Vec<i64>,

    Optional: Option<u64>,
}

#[derive(UnitSection, Debug)]
pub struct OptionalSection {}

#[derive(UnitEntry, Debug)]
enum MyEnum {
    Val1,
    Val2,
}

#[derive(UnitConfig, Debug)]
#[unit(suffix = "unit")]
struct SpecifierUnit {
    #[section(must)]
    Section: SpecifierSection,
}

#[derive(UnitSection, Debug)]
struct SpecifierSection {
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

#[derive(UnitConfig, Debug)]
#[unit(suffix = "unit")]
struct SubdirUnit {
    #[section(must)]
    Section: SubdirSection,
}

#[derive(UnitSection, Debug)]
struct SubdirSection {
    #[entry(multiple, subdir = "wants")]
    Wants: Vec<String>,
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
