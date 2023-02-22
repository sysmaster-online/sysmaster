pub(super) use relation::unit_relation_to_inverse;
pub(super) use relation_atom::unit_relation_from_unique_atom;

// dependency: {ub_relation | ub_relation_atom} -> {ub_load}
mod load;
mod relation;
mod relation_atom;
