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

//! sysmaster-hwdb

use clap::Parser;
use hwdb::HwdbUtil;
use log::Level;

type Result<T> = std::result::Result<T, nix::Error>;

/// update or query the hardware database.
#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(subcommand)]
    subcmd: SubCmd,
}

#[derive(Parser, Debug)]
enum SubCmd {
    /// update hardware database
    #[clap(display_order = 1)]
    Update {
        /// custom .hwdb file path
        #[clap(long, value_parser)]
        path: Option<String>,
        /// generate in /usr/lib/devmaster instead of /etc/devmaster
        #[clap(long, value_parser)]
        usr: bool,
        /// when updating, return non-zero exit value on any parsing error
        #[clap(short, long, value_parser)]
        strict: Option<bool>,
        /// alternative root path in the filesystem
        #[clap(short, long, value_parser)]
        root: Option<String>,
    },
    /// query hardware database
    #[clap(display_order = 2)]
    Query {
        /// device syspath
        #[clap(required = true, value_parser)]
        modalias: String,
        /// alternative root path in the filesystem
        #[clap(short, long, value_parser)]
        root: Option<String>,
    },
}

fn query(modalias: String, root: Option<String>) -> Result<()> {
    HwdbUtil::query(modalias, root)
}

fn update(
    path: Option<String>,
    root: Option<String>,
    usr: bool,
    strict: Option<bool>,
) -> Result<()> {
    let s = strict.unwrap_or(false);
    if usr {
        HwdbUtil::update(
            path,
            root,
            Some("/usr/lib/devmaster/".to_string()),
            s,
            false,
        )
    } else {
        HwdbUtil::update(path, root, None, s, false)
    }
}

fn main() -> Result<()> {
    log::init_log_to_console("sysmaster-hwdb", Level::Debug);
    let args = Args::parse();
    match args.subcmd {
        SubCmd::Query { modalias, root } => query(modalias, root),
        SubCmd::Update {
            path,
            root,
            usr,
            strict,
        } => update(path, root, usr, strict),
    }
}
