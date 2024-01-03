// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

#[cfg(test)]
pub(crate) mod test_utils {
    use std::rc::Rc;

    use crate::{
        unit::{data::DataManager, unit_name_to_type},
        unit::{
            entry::UnitX,
            rentry::UnitRe,
            util::{self, UnitFile},
        },
    };
    use basic::fs::LookupPaths;
    use core::rel::Reliability;
    use core::unit::UmIf;
    pub(crate) struct UmIfD;
    impl UmIf for UmIfD {}

    pub(crate) fn create_unit_for_test_pub(
        dmr: &Rc<DataManager>,
        relir: &Rc<Reliability>,
        rentryr: &Rc<UnitRe>,
        name: &str,
    ) -> Rc<UnitX> {
        let mut l_path = LookupPaths::new();
        let test_units_dir = libtests::get_project_root()
            .unwrap()
            .join("tests/test_units/")
            .to_string_lossy()
            .to_string();
        l_path.search_path.push(test_units_dir);
        let lookup_path = Rc::new(l_path);

        let file = Rc::new(UnitFile::new(&lookup_path));
        let unit_type = unit_name_to_type(name);
        let umifd = Rc::new(UmIfD);
        let subclass = util::create_subunit_with_um(unit_type, umifd).unwrap();
        subclass.attach_reli(Rc::clone(relir));
        Rc::new(UnitX::new(dmr, rentryr, &file, unit_type, name, subclass))
    }
}
