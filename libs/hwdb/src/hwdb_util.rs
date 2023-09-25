use crate::sd_hwdb::{
    SdHwdb, TrieChildEntryF, TrieHeaderF, TrieNodeF, TrieValueEntry2F, TrieValueEntryF,
    HWDB_BIN_PATHS, HWDB_SIG,
};
use basic::strbuf::Strbuf;
use nix::errno::Errno;
use nix::unistd;
use std::cell::RefCell;
use std::fs::{create_dir_all, read_dir, File, Permissions};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::mem::size_of;
use std::os::unix::fs::PermissionsExt;
use std::rc::Rc;

type Result<T> = std::result::Result<T, nix::Error>;

/// path to hwdb.d
pub const CONF_FILE_DIRS: [&str; 2] = ["/etc/devmaster/hwdb.d", "/usr/lib/devmaster/hwdb.d"];

/// in-memory trie objects
struct Trie {
    root: Rc<RefCell<TrieNode>>,
    strings: Strbuf,
    nodes_count: usize,
    children_count: usize,
    values_count: usize,
}

impl Trie {
    fn new() -> Self {
        Trie {
            root: Rc::new(RefCell::new(TrieNode::new())),
            strings: Strbuf::new(),
            nodes_count: 1,
            children_count: 0,
            values_count: 0,
        }
    }

    fn import_file(&mut self, filename: &str, file_priority: u16, compat: bool) -> Result<()> {
        #[derive(PartialEq)]
        enum HWSTATUS {
            None,
            Match,
            Data,
        }
        let mut state = HWSTATUS::None;

        let f = match File::open(filename) {
            Ok(f) => f,
            Err(e) => return Err(Errno::from_i32(e.raw_os_error().unwrap())),
        };

        let mut line_number = 0;
        let mut match_list: Vec<String> = Vec::new();

        let mut reader = BufReader::new(&f);
        loop {
            let mut line = String::new();

            match reader.read_line(&mut line) {
                Ok(size) => {
                    if 0 == size {
                        break;
                    }
                }
                Err(e) => return Err(Errno::from_i32(e.raw_os_error().unwrap())),
            }

            line_number += 1;

            /* comment line */
            if line.starts_with('#') {
                continue;
            }

            /* strip trailing comment */
            if let Some(pos) = line.find('#') {
                line = line[..pos].to_string();
            }

            /* strip trailing whitespace */
            line = line.trim_end().to_string();

            match state {
                HWSTATUS::None => {
                    if line.is_empty() {
                        continue;
                    }

                    if line.starts_with(' ') {
                        log::trace!(
                            "{:?} Match expected but got indented property {:?}, ignoring line.",
                            filename,
                            line
                        );
                        continue;
                    }

                    /* start of record, first match */
                    state = HWSTATUS::Match;

                    match_list.push(line);
                    continue;
                }
                HWSTATUS::Match => {
                    if line.is_empty() {
                        log::trace!(
                            "{:?} Property expected, ignoring record with no properties.",
                            filename
                        );
                        state = HWSTATUS::None;
                        match_list.clear();
                        continue;
                    }

                    if !line.starts_with(' ') {
                        match_list.push(line);
                        continue;
                    }

                    /* first data */
                    state = HWSTATUS::Data;
                    self.insert_data(
                        &match_list,
                        &mut line,
                        filename,
                        file_priority,
                        line_number,
                        compat,
                    );
                    continue;
                }
                HWSTATUS::Data => {
                    if line.is_empty() {
                        /* end of record */
                        state = HWSTATUS::None;
                        match_list.clear();
                        continue;
                    }

                    if !line.starts_with(' ') {
                        log::trace!(
                            "{:?} Property or empty line expected, got {:?}, ignoring record.",
                            filename,
                            line,
                        );
                        state = HWSTATUS::None;
                        match_list.clear();
                        continue;
                    }

                    self.insert_data(
                        &match_list,
                        &mut line,
                        filename,
                        file_priority,
                        line_number,
                        compat,
                    );
                    continue;
                }
            }
        }

        if state == HWSTATUS::Match {
            log::trace!(
                "{:?} Property expected, ignoring record with no properties.",
                filename
            );
        }

        Ok(())
    }

    fn insert_data(
        &mut self,
        match_list: &[String],
        line: &mut String,
        filename: &str,
        file_priority: u16,
        line_number: u32,
        compat: bool,
    ) {
        assert!(line.starts_with(' '));

        let value = match line.find('=') {
            Some(size) => {
                let value = line[size + 1..].to_string();
                *line = line[..size].to_string();
                value
            }
            None => {
                log::warn!(
                    "{:?} Key-value pair expected but got {:?}, ignoring.",
                    filename,
                    line,
                );
                return;
            }
        };

        /* Replace multiple leading spaces by a single space */
        *line = " ".to_string() + &line.trim_start().to_string();

        if line == " " {
            log::warn!(
                "{:?} Empty key in {:?}={:?}, ignoring.",
                filename,
                line,
                value
            );
            return;
        }

        for entry in match_list {
            let value_attr = ValueAttr::new(
                line.to_string(),
                value.clone(),
                filename.to_string(),
                file_priority,
                line_number,
            );
            self.insert(self.root.clone(), entry.as_bytes(), &value_attr, compat);
        }
    }

    fn insert(
        &mut self,
        n: Rc<RefCell<TrieNode>>,
        search: &[u8],
        value_attr: &ValueAttr,
        compat: bool,
    ) {
        let mut i = 0;
        let mut node = n;
        loop {
            let mut p = 0;
            loop {
                let c = self.strings.buf[node.borrow().prefix_off + p];
                if 0 == c {
                    break;
                }

                if c == search[i + p] {
                    p += 1;
                    continue;
                }

                /* split node */
                let mut new_child = TrieNode::new();
                new_child.prefix_off = node.borrow().prefix_off + p + 1;
                new_child.children = node.borrow().children.clone();
                new_child.values = node.borrow().values.clone();

                /* update parent;*/
                let s = self.strings.buf[node.borrow().prefix_off..node.borrow().prefix_off + p]
                    .to_vec();
                let off = self.strings.add_string(s.as_slice());

                *node.borrow_mut() = TrieNode::new();
                node.borrow_mut().prefix_off = off;

                self.node_add_child(node.clone(), Rc::new(RefCell::new(new_child)), c);
                break;
            }
            i += p;

            if search.len() == i {
                return self.node_add_value(node, value_attr, compat);
            }
            let c = search[i];

            node = match node_lookup(node.clone(), c) {
                Some(child) => child,
                None => {
                    let mut new_child = TrieNode::new();
                    let off = self.strings.add_string(&search[i + 1..]);
                    new_child.prefix_off = off;

                    let child = Rc::new(RefCell::new(new_child));
                    self.node_add_child(node, child.clone(), c);

                    return self.node_add_value(child, value_attr, compat);
                }
            };
            i += 1;
        }
    }

    fn node_add_child(
        &mut self,
        node: Rc<RefCell<TrieNode>>,
        node_child: Rc<RefCell<TrieNode>>,
        c: u8,
    ) {
        self.children_count += 1;
        let childentry = TrieChildEntry::new(c, node_child);
        node.borrow_mut().children.push(childentry);
        node.borrow_mut().children.sort_by_key(|c| c.c);
        self.nodes_count += 1;
    }

    fn node_add_value(
        &mut self,
        node: Rc<RefCell<TrieNode>>,
        value_attr: &ValueAttr,
        compat: bool,
    ) {
        let k = self.strings.add_string(value_attr.key.as_bytes());
        let v = self.strings.add_string(value_attr.value.as_bytes());

        let mut f_off = 0;
        if !compat {
            f_off = self.strings.add_string(value_attr.filename.as_bytes());
        }

        let len = node.borrow().values.len();
        if len > 0 {
            let search = TrieValueEntry::new(k, v, 0, 0, 0);
            let val = node
                .borrow()
                .values
                .binary_search_by_key(&search.key_off, |search| search.key_off);
            if let Ok(v) = val {
                node.borrow_mut().values[v].value_off = v;
                node.borrow_mut().values[v].filename_off = f_off;
                node.borrow_mut().values[v].file_priority = value_attr.file_priority;
                node.borrow_mut().values[v].line_number = value_attr.line_number;
                return;
            }
        }

        /* extend array, add new entry, sort for bisection */
        self.values_count += 1;
        let val = TrieValueEntry::new(
            k,
            v,
            f_off,
            value_attr.line_number,
            value_attr.file_priority,
        );
        node.borrow_mut().values.push(val);
        node.borrow_mut()
            .values
            .sort_by_key(|v| self.strings.buf[v.key_off]);
    }

    fn store(&self, filename: String, compat: bool) -> Result<()> {
        let mut t = TrieF::new();
        t.strings_off = size_of::<TrieHeaderF>();

        t.store_nodes_size(&self.root, compat);

        let mut f = match File::create(filename) {
            Ok(f) => f,
            Err(e) => return Err(Errno::from_i32(e.raw_os_error().unwrap())),
        };

        let mut permissions = f.metadata().unwrap().permissions();
        permissions.set_mode(0o444);
        if let Err(e) = f.set_permissions(permissions) {
            return Err(Errno::from_i32(e.raw_os_error().unwrap()));
        }

        let header_size = usize::to_le(size_of::<TrieHeaderF>());
        let node_size = usize::to_le(size_of::<TrieNodeF>());
        let child_entry_size = usize::to_le(size_of::<TrieChildEntryF>());
        let value_entry_size = if compat {
            usize::to_le(size_of::<TrieValueEntryF>())
        } else {
            usize::to_le(size_of::<TrieValueEntry2F>())
        };
        let mut h = TrieHeaderF::new(
            HWDB_SIG,
            0,
            header_size,
            node_size,
            child_entry_size,
            value_entry_size,
        );

        /* write nodes */
        if let Err(e) = f.seek(SeekFrom::Start(size_of::<TrieHeaderF>() as u64)) {
            return Err(Errno::from_i32(e.raw_os_error().unwrap()));
        }

        let root_off = t.store_nodes(&mut f, &self.root, compat)?;
        h.set_nodes_root_off(usize::to_le(root_off));

        let pos = f.seek(SeekFrom::Current(0)).unwrap() as usize;
        h.set_nodes_len(usize::to_le(pos - size_of::<TrieHeaderF>()));

        /* write string buffer */
        f.write_all(&self.strings.buf).unwrap();
        h.set_strings_len(usize::to_le(self.strings.buf.len()));

        /* write header */
        let size = f.seek(SeekFrom::Current(0)).unwrap() as usize;
        h.set_file_size(usize::to_le(size));

        if let Err(e) = f.seek(SeekFrom::Start(0)) {
            return Err(Errno::from_i32(e.raw_os_error().unwrap()));
        }

        let encoded: Vec<u8> = bincode::serialize(&h).unwrap();
        f.write_all(encoded.as_slice()).unwrap();

        /* write succeeded */
        log::debug!("=== trie on-disk ===");
        log::debug!("size:             {:?} bytes", size);
        log::debug!("header:           {:?} bytes", size_of::<TrieHeaderF>());
        log::debug!(
            "nodes:            {:?} bytes ({:?})",
            t.nodes_count * size_of::<TrieNodeF>(),
            t.nodes_count
        );
        log::debug!(
            "child pointers:   {:?} bytes ({:?})",
            t.children_count * size_of::<TrieChildEntryF>(),
            t.children_count
        );
        if compat {
            log::debug!(
                "value pointers:   {:?} bytes ({:?})",
                t.values_count * size_of::<TrieValueEntryF>(),
                t.values_count
            );
        } else {
            log::debug!(
                "value pointers:   {:?} bytes ({:?})",
                t.values_count * size_of::<TrieValueEntry2F>(),
                t.values_count
            );
        }
        log::debug!("string store:     {:?} bytes", self.strings.buf.len());
        log::debug!("strings start:    {:?}", t.strings_off);

        Ok(())
    }
}

struct TrieF {
    strings_off: usize,
    nodes_count: usize,
    children_count: usize,
    values_count: usize,
}
impl TrieF {
    fn new() -> Self {
        TrieF {
            strings_off: 0,
            nodes_count: 0,
            children_count: 0,
            values_count: 0,
        }
    }

    fn store_nodes_size(&mut self, node: &Rc<RefCell<TrieNode>>, compat: bool) {
        for children in node.borrow().children.iter() {
            self.store_nodes_size(&children.child, compat);
        }
        self.strings_off += size_of::<TrieNodeF>();
        self.strings_off += size_of::<TrieChildEntryF>() * node.borrow().children.len();

        if compat {
            self.strings_off += size_of::<TrieValueEntryF>() * node.borrow().values.len();
        } else {
            self.strings_off += size_of::<TrieValueEntry2F>() * node.borrow().values.len();
        }
    }

    fn store_nodes(
        &mut self,
        f: &mut File,
        node: &Rc<RefCell<TrieNode>>,
        compat: bool,
    ) -> Result<usize> {
        let mut children: Vec<TrieChildEntryF> =
            vec![TrieChildEntryF::new(0, 0); node.borrow().children.len()];
        for i in 0..node.borrow().children.len() {
            let child_off = match self.store_nodes(f, &node.borrow().children[i].child, compat) {
                Ok(child_off) => child_off,
                Err(e) => return Err(e),
            };

            children[i] =
                TrieChildEntryF::new(node.borrow().children[i].c, usize::to_le(child_off));
        }

        let n = TrieNodeF::new(
            usize::to_le(self.strings_off + node.borrow().prefix_off),
            node.borrow().children.len() as u8,
            usize::to_le(node.borrow().values.len()),
        );

        /* write node */
        let node_off = f.seek(SeekFrom::Current(0)).unwrap() as usize;

        let encoded: Vec<u8> = bincode::serialize(&n).unwrap();
        f.write_all(encoded.as_slice()).unwrap();
        self.nodes_count += 1;

        /* append children array */
        for child in children.iter().take(node.borrow().children.len()) {
            let encoded: Vec<u8> = bincode::serialize(&child).unwrap();
            f.write_all(encoded.as_slice()).unwrap();
        }
        self.children_count += node.borrow().children.len();

        /* append values array */
        for i in 0..node.borrow().values.len() {
            let key_off = usize::to_le(self.strings_off + node.borrow().values[i].key_off);
            let value_off = usize::to_le(self.strings_off + node.borrow().values[i].value_off);
            if compat {
                let v = TrieValueEntryF::new(key_off, value_off);
                let encoded: Vec<u8> = bincode::serialize(&v).unwrap();
                f.write_all(encoded.as_slice()).unwrap();
            } else {
                let filename_off =
                    usize::to_le(self.strings_off + node.borrow().values[i].filename_off);
                let line_number = u32::to_le(node.borrow().values[i].line_number);
                let file_priority = u16::to_le(node.borrow().values[i].file_priority);
                let v = TrieValueEntry2F::new(
                    key_off,
                    value_off,
                    filename_off,
                    line_number,
                    file_priority,
                );
                let encoded: Vec<u8> = bincode::serialize(&v).unwrap();
                f.write_all(encoded.as_slice()).unwrap();
            };
        }
        self.values_count += node.borrow().values.len();

        Ok(node_off)
    }
}

struct ValueAttr {
    key: String,
    value: String,
    filename: String,
    file_priority: u16,
    line_number: u32,
}
impl ValueAttr {
    fn new(
        key: String,
        value: String,
        filename: String,
        file_priority: u16,
        line_number: u32,
    ) -> Self {
        ValueAttr {
            key,
            value,
            filename,
            file_priority,
            line_number,
        }
    }
}

#[derive(Default, Clone, Debug)]
struct TrieNode {
    /// prefix, common part for all children of this node
    prefix_off: usize,

    /// sorted array of pointers to children nodes
    children: Vec<TrieChildEntry>,

    /// sorted array of key-value pairs
    values: Vec<TrieValueEntry>,
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            prefix_off: 0,
            children: Vec::new(),
            values: Vec::new(),
        }
    }
}

/// children array item with char (0-255) index
#[derive(Clone, Debug)]
struct TrieChildEntry {
    c: u8,
    child: Rc<RefCell<TrieNode>>,
}

impl TrieChildEntry {
    fn new(c: u8, child: Rc<RefCell<TrieNode>>) -> Self {
        TrieChildEntry { c, child }
    }
}

/// value array item with key-value pairs
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct TrieValueEntry {
    key_off: usize,
    value_off: usize,
    filename_off: usize,
    line_number: u32,
    file_priority: u16,
}

impl TrieValueEntry {
    fn new(
        key_off: usize,
        value_off: usize,
        filename_off: usize,
        line_number: u32,
        file_priority: u16,
    ) -> Self {
        TrieValueEntry {
            key_off,
            value_off,
            filename_off,
            line_number,
            file_priority,
        }
    }
}

/// HwdbUtil
pub struct HwdbUtil;
impl HwdbUtil {
    /// update hwdb.bin
    pub fn update(
        path: Option<String>,
        root: Option<String>,
        hwdb_bin_dir: Option<String>,
        strict: bool,
        compat: bool,
    ) -> Result<()> {
        let mut bin_dir = String::from("");
        if let Some(bin) = root {
            bin_dir = bin;
        }

        match hwdb_bin_dir {
            Some(dir) => bin_dir += &dir,
            None => bin_dir += "/etc/devmaster/",
        }
        let hwdb_bin = bin_dir.clone() + "hwdb.bin";

        let mut conf_file_dirs: Vec<String> = Vec::new();
        if let Some(p) = path {
            conf_file_dirs.push(p);
        }
        conf_file_dirs.extend(CONF_FILE_DIRS.iter().map(|s| s.to_string()));

        let mut files = find_files(conf_file_dirs);

        files.sort();
        if files.is_empty() {
            match unistd::unlink(hwdb_bin.as_str()) {
                Ok(()) => log::info!(
                    "No hwdb files found, compiled hwdb database {:?} removed.",
                    hwdb_bin
                ),
                Err(e) => {
                    if e == nix::Error::ENOENT {
                        log::error!(
                            "Failed to remove compiled hwdb database {:?}:{:?}",
                            hwdb_bin,
                            e
                        );
                        return Err(e);
                    }
                    log::info!("No hwdb files found, skipping.");
                }
            }
            return Ok(());
        }

        let mut trie = Trie::new();
        let mut file_priority = 1;
        let mut res = Ok(());

        for file in files {
            log::debug!("Reading file {:?}", file);
            if let Err(e) = trie.import_file(&file, file_priority, compat) {
                if strict {
                    res = Err(e);
                }
            }
            file_priority += 1;
        }

        log::debug!("=== trie in-memory ===");
        log::debug!(
            "nodes:            {:?} bytes ({:?})",
            trie.nodes_count * size_of::<TrieNode>(),
            trie.nodes_count
        );
        log::debug!(
            "children arrays:  {:?} bytes ({:?})",
            trie.children_count * size_of::<TrieChildEntry>(),
            trie.children_count
        );
        log::debug!(
            "values arrays:    {:?} bytes ({:?})",
            trie.values_count * size_of::<TrieValueEntry>(),
            trie.values_count
        );
        log::debug!("strings:          {:?} bytes", trie.strings.buf.len());

        let permissions = Permissions::from_mode(0o755);
        create_dir_all(&bin_dir).unwrap();
        std::fs::set_permissions(&bin_dir, permissions).unwrap();

        trie.store(hwdb_bin, compat)?;

        res
    }

    /// query properties by modalias
    pub fn query(modalias: String, root: Option<String>) -> Result<()> {
        let mut hwdb: SdHwdb;
        if root.is_some() {
            let mut h = Err(Errno::EINVAL);
            for p in HWDB_BIN_PATHS {
                let hwdb_bin = root.clone().unwrap() + p;
                h = SdHwdb::new_from_path(&hwdb_bin);
                if h.is_ok() {
                    break;
                }
            }
            match h {
                Ok(h) => hwdb = h,
                Err(e) => return Err(e),
            }
        } else {
            hwdb = SdHwdb::new()?;
        }

        let map = hwdb.get_properties(modalias)?;
        for it in map.iter() {
            println!("{}={}", it.0, it.1);
        }

        Ok(())
    }
}

fn find_files(conf_file_dirs: Vec<String>) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    for d in conf_file_dirs {
        if let Ok(dir) = read_dir(d) {
            for entry in dir.flatten() {
                let p = entry.path();
                if p.is_file() {
                    let file_name = p.display().to_string();
                    if file_name.ends_with(".hwdb") && !files.contains(&file_name.to_string()) {
                        files.push(file_name.to_string());
                    }
                }
            }
        }
    }
    files
}

fn node_lookup(node: Rc<RefCell<TrieNode>>, c: u8) -> Option<Rc<RefCell<TrieNode>>> {
    let search = TrieChildEntry::new(c, Rc::default());

    match node
        .borrow()
        .children
        .binary_search_by_key(&search.c, |search| search.c)
    {
        Ok(size) => Some(node.borrow().children[size].child.clone()),
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_update() {
        for dir in CONF_FILE_DIRS {
            if Path::new(dir).exists() {
                HwdbUtil::update(None, None, None, false, false).unwrap();
                return;
            }
        }
    }

    #[test]
    fn test_query() {
        for hwdb_bin in HWDB_BIN_PATHS {
            if Path::new(hwdb_bin).exists() {
                HwdbUtil::query("mouse:usb:v3057p0001:".to_string(), None).unwrap();
                return;
            }
        }
    }
}
