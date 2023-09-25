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
//!

use std::cell::RefCell;
use std::rc::Rc;

/// Strbuf stores given strings in a single continuous allocated memory
/// area. Identical strings are de-duplicated and return the same offset
/// as the first string stored. If the tail of a string already exists
/// in the buffer, the tail is returned.
#[derive(Default, Clone, Debug)]
pub struct Strbuf {
    /// store all strings
    pub buf: Vec<u8>,
    root: Rc<RefCell<StrbufNode>>,
    nodes_count: usize,
    in_count: usize,
    in_len: usize,
    dedup_len: usize,
    dedup_count: usize,
}

impl Strbuf {
    /// Create a new Strbuf
    pub fn new() -> Self {
        let buf = vec![0];
        Strbuf {
            buf,
            root: Rc::new(RefCell::new(StrbufNode::new())),
            nodes_count: 1,
            in_count: 0,
            in_len: 0,
            dedup_len: 0,
            dedup_count: 0,
        }
    }

    /// add a string to Strbuf
    pub fn add_string(&mut self, s: &[u8]) -> usize {
        let len = s.len();
        let mut c = 0;

        /* search string; start from last character to find possibly matching tails */
        self.in_count += 1;
        if 0 == len {
            self.dedup_count += 1;
            return 0;
        }
        self.in_len += len;

        let mut node = self.root.clone();
        let mut off;
        for depth in 0..=len {
            /* match against current node */
            let offset: i64 =
                node.borrow().value_off as i64 + node.borrow().value_len as i64 - len as i64;
            off = offset as usize;
            if depth == len
                || (node.borrow().value_len >= len && self.buf[off..].starts_with(&s.to_vec()))
            {
                self.dedup_len += len;
                self.dedup_count += 1;
                return off;
            }

            c = s[len - 1 - depth];

            /* lookup child node */
            let mut search = StrbufChildEntry::new();
            search.c = c;

            node = match node
                .clone()
                .borrow()
                .children
                .binary_search_by_key(&search.c, |search| search.c)
            {
                Ok(size) => node.borrow().children[size].child.clone(),
                Err(_) => break,
            };
        }

        /* add new string */
        off = self.buf.len();
        self.buf.append(&mut s.to_vec());
        self.buf.push(0);

        /* new node */
        let mut node_child = StrbufNode::new();
        node_child.value_off = off;
        node_child.value_len = len;

        /* extend array, add new entry, sort for bisection */
        self.nodes_count += 1;
        node.borrow_mut().bubbleinsert(c, node_child);
        off
    }
}

#[derive(Default, Clone, Debug)]
struct StrbufNode {
    value_off: usize,
    value_len: usize,
    children: Vec<StrbufChildEntry>,
}

impl StrbufNode {
    fn new() -> Self {
        StrbufNode {
            value_off: 0,
            value_len: 0,
            children: Vec::new(),
        }
    }

    fn bubbleinsert(&mut self, c: u8, node_child: StrbufNode) {
        let mut new_child = StrbufChildEntry::new();
        new_child.c = c;
        new_child.child = Rc::new(RefCell::new(node_child));

        let len = self.children.len();

        let mut left = 0;
        let mut right = len;

        while right > left {
            let middle = (right + left) / 2;
            if self.children[middle].cmp(new_child.clone()) <= 0 {
                left = middle + 1;
            } else {
                right = middle;
            }
        }

        if len > left {
            let child = &self.children.clone()[left..len];
            self.children.splice(left + 1..len, child.to_vec());
            self.children[left] = new_child;
        } else {
            self.children.push(new_child);
        }
    }
}

#[derive(Default, Clone, Debug)]
struct StrbufChildEntry {
    c: u8,
    child: Rc<RefCell<StrbufNode>>,
}

impl StrbufChildEntry {
    fn new() -> Self {
        StrbufChildEntry {
            c: 0,
            child: Rc::new(RefCell::new(StrbufNode::new())),
        }
    }

    fn cmp(&self, n: StrbufChildEntry) -> i32 {
        self.c as i32 - n.c as i32
    }
}
