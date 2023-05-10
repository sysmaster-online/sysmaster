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
                $t.rule_file,
                $t.line_number,
                $t.content,
                err
            );
            Error::RulesExecuteError {
                msg: format!("Apply '{}' error: {}", $t.content, err),
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
                        $t.rule_file,
                        $t.line_number,
                        $t.content,
                        err
                    );
                    Err(Error::RulesExecuteError {
                        msg: format!("Apply '{}' error: {}", $t.content, err),
                        errno: err.get_errno(),
                    })
                }
            }
        }
    };
}

/// translate execution error on none return from downside call chain
#[macro_export]
macro_rules! execute_none {
    ($t:expr, $e:expr, $v:expr) => {
        if $e.is_none() {
            log::error!(
                "{}:{}:'{}' {}",
                $t.rule_file,
                $t.line_number,
                $t.content,
                format!("failed to get {}", $v)
            );
            Err(Error::RulesExecuteError {
                msg: format!("Apply '{}' error: have no {}", $t, $v),
                errno: Errno::EINVAL,
            })
        } else {
            Ok($e.unwrap().to_string())
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

/// log info message for rule token
#[macro_export]
macro_rules! log_rule_token_info {
    ($t:expr, $m:expr) => {
        log::info!(
            "{}:{}:'{}' {}",
            $t.rule_file,
            $t.line_number,
            $t.content,
            $m
        );
    };
}

/// log debug message for rule token
#[macro_export]
macro_rules! log_rule_token_debug {
    ($t:expr, $m:expr) => {
        log::debug!(
            "{}:{}:'{}' {}",
            $t.rule_file,
            $t.line_number,
            $t.content,
            $m
        );
    };
}

/// log error message for rule token
#[macro_export]
macro_rules! log_rule_token_error {
    ($t:expr, $m:expr) => {
        log::error!(
            "{}:{}:'{}' {}",
            $t.rule_file,
            $t.line_number,
            $t.content,
            $m
        );
    };
}
