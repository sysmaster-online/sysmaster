#[allow(dead_code)]
enum UnitNameFlags {
    Plain = 1,
    Instance = 2,
    Template = 4,
    Any = 1 | 2 | 4,
}

#[allow(dead_code)]
enum UnitFileState {
    Enabled,
    EnabledRuntime,
    Linked,
    LinkedRuntime,
    Alias,
    Masked,
    MaskedRuntime,
    Static,
    Disabled,
    Indirect,
    Generated,
    Transient,
    Bad,
}
