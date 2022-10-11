#![warn(unused_imports)]

#[allow(clippy::enum_variant_names)]
#[derive(PartialEq, Debug, Eq, Copy, Clone)]
pub(in crate::manager::unit) enum UnitLoadState {
    UnitStub = 0,
    UnitLoaded,
    UnitNotFound,
    UnitError,
    UnitMerged,
    UnitMasked,
}

#[allow(clippy::enum_variant_names)]
enum UnitNameFlags {
    UnitNamePlain = 1,
    UnitNameInstance = 2,
    UnitNameTemplate = 4,
    UnitNameAny = 1 | 2 | 4,
}

#[allow(clippy::enum_variant_names)]
enum UnitFileState {
    UnitFileEnabled,
    UnitFileEnabledRuntime,
    UnitFileLinked,
    UnitFileLinkedRuntime,
    UnitFileAlias,
    UnitFileMasked,
    UnitFileMaskedRuntime,
    UnitFileStatic,
    UnitFileDisabled,
    UnitFileIndirect,
    UnitFileGenerated,
    UnitFileTransient,
    UnitFileBad,
}
