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

use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, RwLock},
};

#[derive(Debug)]
pub(crate) struct Trie<E>
where
    E: FromStr,
{
    pub(crate) children: HashMap<char, Arc<RwLock<Trie<E>>>>,
    pub(crate) is_end: bool,
    pub(crate) value: Option<E>,
    pub(crate) depth: usize,
}

impl<E> Trie<E>
where
    E: FromStr,
{
    pub(crate) fn from_vec(v: Vec<&str>) -> Arc<RwLock<Self>> {
        let ret = Arc::new(RwLock::new(Self::new(0)));

        for s in v {
            Self::insert(ret.clone(), s);
        }

        ret
    }

    pub(crate) fn new(depth: usize) -> Self {
        Self {
            children: HashMap::new(),
            is_end: false,
            value: None,
            depth,
        }
    }

    /// Search the prefix as far as possible.
    pub(crate) fn search_prefix_partial(
        mut node: Arc<RwLock<Trie<E>>>,
        whole: &str,
    ) -> Option<Arc<RwLock<Trie<E>>>> {
        for ch in whole.chars() {
            let child = node.read().unwrap().children.get(&ch).cloned();
            match child {
                Some(n) => {
                    node = n.clone();
                }
                None => {
                    if node.read().unwrap().is_end {
                        break;
                    }
                    return None;
                }
            }
        }

        Some(node)
    }

    #[allow(dead_code)]
    pub(crate) fn search_prefix(
        mut node: Arc<RwLock<Trie<E>>>,
        prefix: &str,
    ) -> Option<Arc<RwLock<Trie<E>>>> {
        for ch in prefix.chars() {
            let child = node.read().unwrap().children.get(&ch).cloned();
            match child {
                Some(n) => {
                    node = n.clone();
                }
                None => {
                    return None;
                }
            }
        }

        Some(node)
    }

    pub(crate) fn insert(mut node: Arc<RwLock<Trie<E>>>, word: &str) {
        for ch in word.chars() {
            let child = node.read().unwrap().children.get(&ch).cloned();
            match child {
                Some(n) => {
                    node = n;
                }
                None => {
                    let n = Arc::new(RwLock::new(Self::new(node.read().unwrap().depth + 1)));
                    let _ = node.write().unwrap().children.insert(ch, n.clone());
                    node = n;
                }
            }
        }

        node.write().unwrap().is_end = true;
        node.write().unwrap().value = word.parse::<E>().ok();
    }

    #[allow(dead_code)]
    pub(crate) fn search(node: Arc<RwLock<Trie<E>>>, word: &str) -> bool {
        let ret = Self::search_prefix(node, word);
        ret.is_some() && ret.unwrap().read().unwrap().is_end
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_trie() {
        let root = Arc::new(RwLock::new(Trie::<String>::new(0)));
        Trie::insert(root.clone(), "hello");
        Trie::insert(root.clone(), "world");
        Trie::insert(root.clone(), "hell");

        assert!(Trie::search(root.clone(), "hello"));
        assert!(Trie::search(root.clone(), "world"));
        assert!(Trie::search(root.clone(), "hell"));
        assert!(!Trie::search(root.clone(), "water"));
        let world = Trie::search_prefix(root.clone(), "world");
        assert!(world.is_some());
        assert_eq!(world.unwrap().read().unwrap().depth, 5);
        assert!(Trie::search_prefix(root.clone(), "hell").is_some());
        assert!(Trie::search_prefix(root.clone(), "helloworld").is_none());
        assert!(Trie::search_prefix_partial(root, "helloworld").is_some());
    }
}
