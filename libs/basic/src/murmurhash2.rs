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

// Consistent with systemd
const M: u32 = 0x5bd1e995;
const R: i32 = 24;

/// Compatible with 'MurmurHash2' in libsystemd
pub fn murmurhash2(key: &[u8], len: usize, seed: u32) -> u32 {
    let mut h = seed ^ (len as u32);
    let mut idx: usize = 0;

    while len - idx >= 4 {
        let mut k = load_u32(key, len, idx);

        k = k.wrapping_mul(M);
        k ^= k >> R;
        k = k.wrapping_mul(M);

        h = h.wrapping_mul(M);
        h ^= k;

        idx += 4;
    }

    if len - idx > 0 {
        for i in 0..len - idx {
            h ^= (*key.get(idx + (i as usize)).unwrap() as u32) << (8 * i);
        }
        h = h.wrapping_mul(M);
    }

    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^= h >> 15;

    h
}

#[inline]
fn load_u32(key: &[u8], len: usize, idx: usize) -> u32 {
    assert!(idx + 4 <= len);

    let s: [u8; 4] = [
        *key.get(idx).unwrap(),
        *key.get(idx + 1).unwrap(),
        *key.get(idx + 2).unwrap(),
        *key.get(idx + 3).unwrap(),
    ];

    unsafe { std::mem::transmute::<[u8; 4], u32>(s) }
}

#[cfg(test)]
mod tests {
    use super::murmurhash2;

    #[test]
    fn test_murmurhash2() {
        let h1 = murmurhash2("test".as_bytes(), 4, 0);
        let h2 = murmurhash2("test1".as_bytes(), 5, 0);
        let h3 = murmurhash2("test12".as_bytes(), 6, 0);
        let h4 = murmurhash2("test123".as_bytes(), 7, 0);
        assert!(h1 > 0);
        assert!(h2 > 0);
        assert!(h3 > 0);
        assert!(h4 > 0);
    }
}
