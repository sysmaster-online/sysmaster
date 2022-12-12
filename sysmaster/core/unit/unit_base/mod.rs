pub(super) use ub_relation::unit_relation_to_inverse;
pub(super) use ub_relation_atom::unit_relation_from_unique_atom;

// dependency: {ub_relation | ub_relation_atom} -> {ub_load}
mod ub_load;
mod ub_relation;
mod ub_relation_atom;
