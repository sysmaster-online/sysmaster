#![warn(unused_imports)]

#[derive(PartialEq, Debug, Eq, Copy, Clone)]
pub(in crate::manager::unit) enum UnitLoadState {
    UnitStub = 0,
    UnitLoaded,
    UnitNotFound,
    UnitError,
    UnitMerged,
    UnitMasked,
}

enum UnitNameFlags {
    UnitNamePlain = 1,
    UnitNameInstance = 2,
    UnitNameTemplate = 4,
    UnitNameAny = 1 | 2 | 4,
}

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
