use basic::{fd_util, unistd::timespec_load};
use constants::INVALID_FD;
use glob::Pattern;
use linked_hash_map::LinkedHashMap;
use memoffset::offset_of;
use nix::errno::Errno;
use nix::sys::stat::{fstat, FileStat};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{Read, Seek};
use std::os::unix::prelude::AsRawFd;
use std::time::{Duration, UNIX_EPOCH};

type Result<T> = std::result::Result<T, nix::Error>;

/// path to hwdb.bin
pub const HWDB_BIN_PATHS: [&str; 4] = [
    "/etc/sysmaster/hwdb/hwdb.bin",
    "/etc/devmaster/hwdb.bin",
    "/usr/lib/sysmaster/hwdb/hwdb.bin",
    "/usr/lib/devmaster/hwdb.bin",
];

/// hwdb verification head
pub const HWDB_SIG: [u8; 8] = [b'K', b'S', b'L', b'P', b'H', b'H', b'R', b'H'];

#[repr(C)]
/// SdHwdb
pub struct SdHwdb {
    n_ref: u32,
    f: File,
    st: FileStat,
    head: TrieHeaderF,
    map: Vec<u8>,
    properties: LinkedHashMap<String, TrieValueEntry2F>,
    properties_modified: bool,
}

#[repr(C)]
#[derive(Serialize, Deserialize)]
/// TrieHeaderF
pub struct TrieHeaderF {
    signature: [u8; 8],

    /// version of tool which created the file
    tool_version: usize,
    file_size: usize,

    /// size of structures to allow them to grow
    header_size: usize,
    node_size: usize,
    child_entry_size: usize,
    value_entry_size: usize,

    /// offset of the root trie node
    nodes_root_off: usize,

    /// size of the nodes and string section
    nodes_len: usize,
    strings_len: usize,
}

impl TrieHeaderF {
    /// create TrieHeaderF
    pub fn new(
        signature: [u8; 8],
        tool_version: usize,
        header_size: usize,
        node_size: usize,
        child_entry_size: usize,
        value_entry_size: usize,
    ) -> Self {
        TrieHeaderF {
            signature,
            tool_version,
            file_size: 0,
            header_size,
            node_size,
            child_entry_size,
            value_entry_size,
            nodes_root_off: 0,
            nodes_len: 0,
            strings_len: 0,
        }
    }

    /// set nodes_root_off
    pub fn set_nodes_root_off(&mut self, nodes_root_off: usize) {
        self.nodes_root_off = nodes_root_off;
    }

    /// set nodes_len
    pub fn set_nodes_len(&mut self, nodes_len: usize) {
        self.nodes_len = nodes_len;
    }

    /// set strings_len
    pub fn set_strings_len(&mut self, strings_len: usize) {
        self.strings_len = strings_len;
    }

    /// set file_size
    pub fn set_file_size(&mut self, file_size: usize) {
        self.file_size = file_size;
    }
}

#[repr(C)]
#[derive(Serialize, Deserialize, Debug, Clone)]
/// TrieNodeF
pub struct TrieNodeF {
    /// prefix of lookup string, shared by all children
    pub prefix_off: usize,
    /// size of children entry array appended to the node
    pub children_count: u8,
    padding: [u8; 7],
    /// size of value entry array appended to the node
    pub values_count: usize,
}

impl TrieNodeF {
    /// create TrieNodeF
    pub fn new(prefix_off: usize, children_count: u8, values_count: usize) -> Self {
        TrieNodeF {
            prefix_off,
            children_count,
            padding: [0; 7],
            values_count,
        }
    }
}

#[repr(C)]
#[derive(Serialize, Deserialize, Debug, Clone)]
/// TrieNode
pub struct TrieNode {
    trie_node_f: TrieNodeF,
    node_index: usize,
}

impl TrieNode {
    /// create TrieNode
    pub fn new(trie_node_f: TrieNodeF, node_index: usize) -> Self {
        TrieNode {
            trie_node_f,
            node_index,
        }
    }
}

#[repr(C)]
#[derive(Serialize, Deserialize, Debug, Clone)]
/// array of child entries, follows directly the node record
pub struct TrieChildEntryF {
    /// index of the child node
    c: u8,
    padding: [u8; 7],
    /// offset of the child node
    child_off: usize,
}

impl TrieChildEntryF {
    /// create TrieChildEntryF
    pub fn new(c: u8, child_off: usize) -> Self {
        TrieChildEntryF {
            c,
            padding: [0; 7],
            child_off,
        }
    }
}

#[repr(C)]
#[derive(Serialize, Deserialize, Debug)]
/// array of value entries, follows directly the node record/child array
pub struct TrieValueEntryF {
    key_off: usize,
    value_off: usize,
}

impl TrieValueEntryF {
    /// create TrieValueEntryF
    pub fn new(key_off: usize, value_off: usize) -> Self {
        TrieValueEntryF { key_off, value_off }
    }
}

#[repr(C)]
#[derive(Serialize, Deserialize, Debug)]
/// v2 extends v1 with filename and line-number
pub struct TrieValueEntry2F {
    key_off: usize,
    value_off: usize,
    filename_off: usize,
    line_number: u32,
    file_priority: u16,
    padding: u16,
}

impl TrieValueEntry2F {
    /// create TrieValueEntry2F
    pub fn new(
        key_off: usize,
        value_off: usize,
        filename_off: usize,
        line_number: u32,
        file_priority: u16,
    ) -> Self {
        TrieValueEntry2F {
            key_off,
            value_off,
            filename_off,
            line_number,
            file_priority,
            padding: 0,
        }
    }
}

impl SdHwdb {
    /// create SdHwdb with path
    pub fn new_from_path(path: &str) -> Result<Self> {
        if path.is_empty() {
            return Err(Errno::EINVAL);
        }

        hwdb_new(path)
    }

    /// create SdHwdb
    pub fn new() -> Result<Self> {
        hwdb_new("")
    }

    /// determine if hwdb.bin should reload
    pub fn should_reload(&self) -> bool {
        let mut found: bool = false;

        let duration = Duration::from_secs(self.st.st_mtime as u64);
        let st_time = UNIX_EPOCH + duration;
        let mut time = st_time;

        if self.f.as_raw_fd() == INVALID_FD {
            return false;
        }

        /* if hwdb.bin doesn't exist anywhere, we need to update */
        for p in HWDB_BIN_PATHS {
            if let Ok(s) = std::fs::metadata(p) {
                time = match s.modified() {
                    Ok(t) => t,
                    Err(e) => {
                        log::error!("Failed to get mtime of {:?}:{:?}", p, e);
                        continue;
                    }
                };
                found = true;
                break;
            }
        }

        if !found {
            return true;
        }

        if timespec_load(st_time) != timespec_load(time) {
            return true;
        }

        false
    }

    /// get value by modalias and key
    pub fn sd_hwdb_get(&mut self, modalias: String, key: String) -> Result<String> {
        if let Err(err) = self.properties_prepare(modalias) {
            return Err(err);
        }

        let entry = match self.properties.get(&key) {
            Some(entry) => entry,
            None => return Err(Errno::ENOENT),
        };

        let value = self.trie_string(entry.value_off);

        Ok(value)
    }

    /// get properties by modalias
    pub fn get_properties(&mut self, modalias: String) -> Result<HashMap<String, String>> {
        self.seek(modalias)?;
        self.enumerate()
    }

    fn seek(&mut self, modalias: String) -> Result<()> {
        if let Err(err) = self.properties_prepare(modalias) {
            return Err(err);
        }
        self.properties_modified = false;

        Ok(())
    }

    fn enumerate(&mut self) -> Result<HashMap<String, String>> {
        if self.properties_modified {
            return Err(Errno::EAGAIN);
        }

        let mut map: HashMap<String, String> = HashMap::new();

        for it in self.properties.iter() {
            let key = it.0;
            let entry = it.1;
            let value = self.trie_string(entry.value_off);
            map.insert(key.to_string(), value);
        }

        Ok(map)
    }

    fn properties_prepare(&mut self, modalias: String) -> Result<()> {
        self.properties.clear();
        self.properties_modified = true;
        self.trie_search_f(modalias)
    }

    fn trie_search_f(&mut self, search: String) -> Result<()> {
        let mut i: usize = 0;
        let mut buf = LineBuf::new();
        let mut node = self.trie_node_from_off(self.head.nodes_root_off);

        loop {
            let mut p: usize = 0;
            if 0 != node.trie_node_f.prefix_off {
                loop {
                    let s = self.trie_string(node.trie_node_f.prefix_off);
                    if s.len() == p {
                        break;
                    }

                    let c = s.as_bytes()[p];
                    if c == b'*' || c == b'?' || c == b'[' {
                        return self.trie_fnmatch_f(
                            node,
                            p,
                            &mut buf,
                            &search[i + p..].to_string(),
                        );
                    }

                    if c != search.as_bytes()[i + p] {
                        return Ok(());
                    }
                    p += 1;
                }
                i += p;
            }

            let child = self.node_lookup_f(&node, b'*');
            if let Some(match_child) = child {
                buf.add_char(b'*');
                if let Err(err) =
                    self.trie_fnmatch_f(match_child, 0, &mut buf, &search[i..].to_string())
                {
                    return Err(err);
                }
                buf.rem_char();
            }

            let child = self.node_lookup_f(&node, b'?');
            if let Some(match_child) = child {
                buf.add_char(b'?');
                if let Err(err) =
                    self.trie_fnmatch_f(match_child, 0, &mut buf, &search[i..].to_string())
                {
                    return Err(err);
                }
                buf.rem_char();
            }

            let child = self.node_lookup_f(&node, b'[');
            if let Some(match_child) = child {
                buf.add_char(b'[');
                if let Err(err) =
                    self.trie_fnmatch_f(match_child, 0, &mut buf, &search[i..].to_string())
                {
                    return Err(err);
                }
                buf.rem_char();
            }

            if search.len() == i {
                for n in 0..usize::from_le(node.trie_node_f.values_count) {
                    if let Err(err) = self.add_property(&node, n) {
                        return Err(err);
                    }
                }
                return Ok(());
            }

            let child = self.node_lookup_f(&node, search.as_bytes()[i]);
            if child.is_none() {
                break;
            }
            node = child.unwrap();
            i += 1;
        }
        Ok(())
    }

    fn node_lookup_f(&mut self, node: &TrieNode, c: u8) -> Option<TrieNode> {
        let search = TrieChildEntryF::new(c, 0);

        let mut children: Vec<TrieChildEntryF> = Vec::new();
        for child_count in 0..node.trie_node_f.children_count {
            let off = node.node_index
                + usize::from_le(self.head.node_size)
                + child_count as usize * usize::from_le(self.head.child_entry_size);
            let child: TrieChildEntryF = bincode::deserialize(&self.map[off..]).unwrap();
            children.push(child);
        }

        children.sort_by_key(|search| search.c);
        match children.binary_search_by_key(&search.c, |search| search.c) {
            Ok(a) => Some(self.trie_node_from_off(children[a].child_off)),
            Err(_) => None,
        }
    }

    fn trie_fnmatch_f(
        &mut self,
        node: TrieNode,
        p: usize,
        buf: &mut LineBuf,
        search: &str,
    ) -> Result<()> {
        let prefix = self.trie_string(node.trie_node_f.prefix_off);
        let add_prefix = prefix[p..].to_string();
        buf.add(&add_prefix);
        let len = add_prefix.len();

        for i in 0..node.trie_node_f.children_count {
            let child = self.trie_node_child(node.clone(), i as usize);
            buf.add_char(child.c);
            let f = self.trie_node_from_off(child.child_off);
            if let Err(e) = self.trie_fnmatch_f(f, 0, buf, search) {
                return Err(e);
            }
            buf.rem_char();
        }

        let pattern = Pattern::new(&buf.get()).unwrap();
        if usize::from_le(node.trie_node_f.values_count) > 0 && pattern.matches(search) {
            for i in 0..usize::from_le(node.trie_node_f.values_count) {
                if let Err(e) = self.add_property(&node, i) {
                    return Err(e);
                }
            }
        }

        buf.rem(len);
        Ok(())
    }

    fn trie_node_value(&self, node: &TrieNode, idx: usize) -> TrieValueEntryF {
        let mut off = node.node_index + usize::from_le(self.head.node_size);
        off +=
            node.trie_node_f.children_count as usize * usize::from_le(self.head.child_entry_size);
        off += idx * usize::from_le(self.head.value_entry_size);

        let value: TrieValueEntryF = bincode::deserialize(&self.map[off..]).unwrap();
        value
    }

    fn trie_node_value2(&self, node: &TrieNode, idx: usize) -> TrieValueEntry2F {
        let mut off = node.node_index + usize::from_le(self.head.node_size);
        off +=
            node.trie_node_f.children_count as usize * usize::from_le(self.head.child_entry_size);
        off += idx * usize::from_le(self.head.value_entry_size);

        let value: TrieValueEntry2F = bincode::deserialize(&self.map[off..]).unwrap();
        value
    }

    fn add_property(&mut self, node: &TrieNode, idx: usize) -> Result<()> {
        let entry = self.trie_node_value(node, idx);
        let mut key = self.trie_string(entry.key_off);

        let mut entry2 = TrieValueEntry2F::new(entry.key_off, entry.value_off, 0, 0, 0);

        /*
         * Silently ignore all properties which do not start with a
         * space; future extensions might use additional prefixes.
         */
        if !key.starts_with(' ') {
            return Ok(());
        }

        key.remove(0);

        if usize::from_le(self.head.value_entry_size) >= std::mem::size_of::<TrieValueEntry2F>() {
            entry2 = self.trie_node_value2(node, idx);
            if let Some(old) = self.properties.get(&key) {
                /* On duplicates, we order by filename priority and line-number.
                 *
                 * v2 of the format had 64 bits for the line number.
                 * v3 reuses top 32 bits of line_number to store the priority.
                 * We check the top bits — if they are zero we have v2 format.
                 * This means that v2 clients will print wrong line numbers with
                 * v3 data.
                 *
                 * For v3 data: we compare the priority (of the source file)
                 * and the line number.
                 *
                 * For v2 data: we rely on the fact that the filenames in the hwdb
                 * are added in the order of priority (higher later), because they
                 * are *processed* in the order of priority. So we compare the
                 * indices to determine which file had higher priority. Comparing
                 * the strings alphabetically would be useless, because those are
                 * full paths, and e.g. /usr/lib would sort after /etc, even
                 * though it has lower priority. This is not reliable because of
                 * suffix compression, but should work for the most common case of
                 * /usr/lib/devmaster/hwbd.d and /etc/devmaster/hwdb.d, and is
                 * better than not doing the comparison at all.
                 */
                let lower;
                if entry2.file_priority == 0 {
                    lower = entry2.filename_off < old.filename_off
                        || (entry2.filename_off == old.filename_off
                            && entry2.line_number < old.line_number);
                } else {
                    lower = entry2.file_priority < old.file_priority
                        || (entry2.file_priority == old.file_priority
                            && entry2.line_number < old.line_number);
                }

                if lower {
                    return Ok(());
                }
            }
        }

        self.properties.insert(key, entry2);
        self.properties_modified = true;

        Ok(())
    }

    fn trie_node_child(&self, node: TrieNode, idx: usize) -> TrieChildEntryF {
        let off = node.node_index
            + usize::from_le(self.head.node_size)
            + idx * usize::from_le(self.head.child_entry_size);

        let child: TrieChildEntryF = bincode::deserialize(&self.map[off..]).unwrap();
        child
    }

    fn trie_node_from_off(&mut self, off: usize) -> TrieNode {
        let trie_node_off = usize::from_le(off);
        let trie_node_f: TrieNodeF = bincode::deserialize(&self.map[trie_node_off..]).unwrap();

        TrieNode::new(trie_node_f, trie_node_off)
    }

    fn trie_string(&self, off: usize) -> String {
        let mut s: Vec<u8> = Vec::new();
        let mut i = 0;
        while let Ok(c) = bincode::deserialize::<u8>(&self.map[usize::from_le(off) + i..]) {
            if 0 == c {
                break;
            } else {
                s.push(c);
            }
            i += 1;
        }
        String::from_utf8(s).unwrap()
    }
}

impl Drop for SdHwdb {
    fn drop(&mut self) {
        if nix::unistd::close(self.f.as_raw_fd()).is_err() {
            log::error!("Failed to close fd {:?}", self.f);
        }
    }
}

fn hwdb_new(path: &str) -> Result<SdHwdb> {
    let sig = HWDB_SIG;
    let mut file: Option<File> = None;
    let mut hwdb_path = path;
    let hwdb_st: FileStat;
    /* Find hwdb.bin in the explicit path if provided, or iterate over hwdb_bin_paths otherwise  */
    if !path.is_empty() {
        log::debug!("Trying to open \"{:?}\"...", path);
        file = match OpenOptions::new().read(true).open(path) {
            Ok(f) => Some(f),
            Err(e) => {
                log::error!("Failed to open {:?}", path);
                return Err(Errno::from_i32(e.raw_os_error().unwrap()));
            }
        };
    } else {
        for p in HWDB_BIN_PATHS {
            log::debug!("Trying to open \"{:?}\"...", p);
            let f = OpenOptions::new().read(true).open(p);
            if let Ok(ff) = f {
                file = Some(ff);
                hwdb_path = p;
                break;
            }

            let err = Errno::from_i32(f.err().unwrap().raw_os_error().unwrap());
            if err != Errno::ENOENT {
                log::error!("Failed to open {:?}", p);
                return Err(err);
            }
        }
        if file.is_none() {
            log::error!("hwdb.bin does not exist, please run 'sysmaster-hwdb update'");
            return Err(Errno::ENOENT);
        }
    }
    let mut hwdb_file = file.unwrap();

    hwdb_st = match fstat(hwdb_file.as_raw_fd()) {
        Ok(st) => st,
        Err(e) => {
            log::error!("Failed to fstat {:?} : {:?}", hwdb_path, e);
            return Err(e);
        }
    };

    if hwdb_st.st_size < offset_of!(TrieHeaderF, strings_len) as i64 + 8 {
        log::error!("File{:?} is too short", hwdb_path);
        return Err(Errno::EIO);
    }
    if fd_util::file_offset_beyond_memory_size(hwdb_st.st_size) {
        log::error!("File {:?} is too long", hwdb_path);
        return Err(Errno::EFBIG);
    }

    hwdb_file.seek(std::io::SeekFrom::Start(0)).unwrap();
    let mut buffer: [u8; 1024] = [0; 1024];
    let mut hwdb_map = Vec::new();
    loop {
        let n = hwdb_file.read(&mut buffer).unwrap();
        if 0 == n {
            break;
        }
        hwdb_map.extend_from_slice(&buffer[..n]);
    }

    let hwdb_head: TrieHeaderF = bincode::deserialize(&hwdb_map).unwrap();
    if hwdb_head.signature != sig || hwdb_st.st_size as usize != usize::from_le(hwdb_head.file_size)
    {
        log::error!("Failed to recognize the format of {:?}", hwdb_path);
        return Err(Errno::EINVAL);
    }

    log::debug!("=== trie on-disk ===");
    log::debug!(
        "tool version:          {:?}",
        usize::from_le(hwdb_head.tool_version)
    );
    log::debug!("file size:        {:?} bytes", hwdb_st.st_size);
    log::debug!(
        "header size       {:?} bytes",
        usize::from_le(hwdb_head.header_size)
    );
    log::debug!(
        "strings           {:?} bytes",
        usize::from_le(hwdb_head.strings_len)
    );
    log::debug!(
        "nodes             {:?} bytes",
        usize::from_le(hwdb_head.nodes_len)
    );

    Ok(SdHwdb {
        n_ref: 1,
        f: hwdb_file,
        st: hwdb_st,
        head: hwdb_head,
        map: hwdb_map,
        properties: LinkedHashMap::new(),
        properties_modified: false,
    })
}

struct LineBuf {
    bytes: String,
    max_len: usize,
}

impl LineBuf {
    fn new() -> Self {
        LineBuf {
            bytes: "".to_string(),
            max_len: 2048,
        }
    }

    fn get(&self) -> String {
        if self.bytes.len() + 1 >= self.max_len {
            return "".to_string();
        }

        self.bytes.clone()
    }

    fn add(&mut self, s: &str) -> bool {
        if self.bytes.len() + s.len() >= self.max_len {
            return false;
        }
        self.bytes += s;
        true
    }

    fn add_char(&mut self, c: u8) -> bool {
        if self.bytes.len() + 1 >= self.max_len {
            return false;
        }
        self.bytes.push(char::from(c));
        true
    }

    fn rem(&mut self, count: usize) {
        assert!(self.bytes.len() >= count);

        self.bytes.truncate(self.bytes.len() - count);
    }

    fn rem_char(&mut self) {
        self.rem(1);
    }
}
