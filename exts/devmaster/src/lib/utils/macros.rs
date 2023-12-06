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

//! macro utilities
//!

/// translate execution error from downside call chain
#[macro_export]
macro_rules! execute_err {
    ($t:expr, $e:expr) => {
        $e.map_err(|err| {
            log::error!(
                "{}:{}:'{}' {}",
                $t.get_file_name(),
                $t.get_line_number(),
                $t,
                err
            );
            Error::RulesExecuteError {
                msg: format!("Apply '{}' error: {}", $t, err),
                errno: err.get_errno(),
            }
        })
    };
}

/// translate execution error from downside call chain
#[macro_export]
macro_rules! execute_err_ignore_ENOENT {
    ($t:expr, $e:expr) => {
        match $e {
            Ok(ret) => Ok(ret.to_string()),
            Err(err) => {
                if err.get_errno() == Errno::ENOENT {
                    Ok(String::new())
                } else {
                    log::error!(
                        "{}:{}:'{}' {}",
                        $t.get_file_name(),
                        $t.get_line_number(),
                        $t,
                        err
                    );
                    Err(Error::RulesExecuteError {
                        msg: format!("Apply '{}' error: {}", $t, err),
                        errno: err.get_errno(),
                    })
                }
            }
        }
    };
}

/// translate substitute formatter error into execute error
#[macro_export]
macro_rules! subst_format_map_err {
    ($e:expr, $k:expr) => {
        match $e {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::RulesExecuteError {
                msg: format!("failed to substitute formatter '{}': ({})", $k, e),
                source: e.get_errno(),
            }),
        }
    };
}

/// translate substitute formatter error, unless it is ignored, into execute error
#[macro_export]
macro_rules! subst_format_map_err_ignore {
    ($e:expr, $k:expr, $i:expr, $d:expr) => {
        match $e {
            Ok(v) => Ok(v),
            Err(e) => {
                if e.get_errno() == $i {
                    Ok($d)
                } else {
                    Err(Error::RulesExecuteError {
                        msg: format!("failed to substitute formatter '{}': ({})", $k, e),
                        errno: e.get_errno(),
                    })
                }
            }
        }
    };
}

/// translate substitute formatter error on none into execute error
#[macro_export]
macro_rules! subst_format_map_none {
    ($e:expr, $k:expr, $d:expr) => {
        match $e {
            Some(v) => Ok(v.to_string()),
            None => {
                log::debug!("formatter '{}' got empty value.", $k);
                Ok($d)
            }
        }
    };
}

/// log message for rule token
#[macro_export]
macro_rules! log_rule_token {
    ($l:ident, $t:expr, $m:expr) => {
        log::$l!(
            "{}:{}:'{}' {}",
            $t.get_file_name(),
            $t.get_line_number(),
            $t,
            $m
        )
    };
}

/// log message for rule line
#[macro_export]
macro_rules! log_rule_line {
    ($l:ident, $t:expr, $m:expr) => {
        log::$l!("{}:{}: {}", $t.get_file_name(), $t.line_number, $m)
    };
}

/// log message about device
#[macro_export]
macro_rules! log_dev {
    ($level:ident, $dev:expr, $msg:expr) => {
        log::$level!("{}: {}", $dev.get_sysname().unwrap_or_default(), $msg)
    };
}

/// log message about device
#[macro_export]
macro_rules! log_dev_option {
    ($level:ident, $dev:expr, $msg:expr) => {
        match $dev {
            Some(d) => {
                $crate::log_dev!($level, d, $msg);
            }
            None => {
                log::$level!("{}", $msg);
            }
        }
    };
}
