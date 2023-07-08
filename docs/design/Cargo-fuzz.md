# Cargo-fuzz 介绍

## 什么是 Fuzz

Fuzz （模糊测试）是一种软件测试技术。其核心思想是将自动或半自动生成的随机数据输入到一个程序中，并监视程序异常，如崩溃，断言（assertion）失败，以发现可能的程序错误，比如内存泄漏。模糊测试常常用于检测软件或计算机系统的安全漏洞。

## Cargo-fuzz 简介

[Cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) 是用于模糊测试 Rust 代码的推荐工具。

Cargo-fuzz 本身不是 fuzzer，而是调用 fuzzer 的工具。目前，它唯一支持的 fuzzer 是 [libFuzzer](http://llvm.org/docs/LibFuzzer.html)（使用 [libfuzzer-sys](https://github.com/rust-fuzz/libfuzzer-sys) crate), 但它可以[扩展到未来支持其他 fuzzer](https://github.com/rust-fuzz/cargo-fuzz/issues/1)。libFuzzer 需要 LLVM sanitizer 程序支持，因此目前这只适用于 x86-64 Linux、x86-64 macOS 和 Apple-Silicon (aarch64) macOS。

## Cargo-fuzz 安装

因为模糊测试要使用 `-Z` 编译器标志来提供地址 sanitization，因此需要安装 nightly 编译器。

安装并设置使用 nightly 编译器

```
rustup override set nightly
```

安装 cargo-fuzz

```
cargo install cargo-fuzz
```

## Cargo-fuzz 使用

以 [rust-url](https://github.com/servo/rust-url) 项目为例，克隆对应代码并 checkout 指定版本

```
git clone https://github.com/servo/rust-url.git
cd rust-url
git checkout bfa167b4e0253642b6766a7aa74a99df60a94048
```

1、初始化 cargo-fuzz（主动指定 target 为 fuzz_test ）

```
cargo fuzz init -t fuzz_test
```

2、编写 fuzz 测试程序

```
// fuzz/fuzz_targets/fuzz_test.rs

#![no_main]

extern crate libfuzzer_sys;
extern crate url;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = url::Url::parse(s);
    }
});
```

3、查看当前存在的 fuzz target 可以确认对应的 target 名称为 fuzz_test

```
cargo fuzz list
```

4、运行 fuzz 测试（该测试测序在运行一定时间后会出现 panic）

```
cargo fuzz run fuzz_test
```

## Structure-Aware Fuzzing

观察前文 fuzz 测试程序可以发现，fuzz 测试程序的入参是字符串类型的，但事实上并非每个 fuzz target 都希望将字符串作为输入。幸运的是，只要实现了[`Arbitrary` trait](https://docs.rs/arbitrary/*/arbitrary/trait.Arbitrary.html)，`libfuzzer-sys` crate 可以让我们定义任何类型的 fuzz target。以下以自定义的 alloc 分配器为例进行说明：

1、创建 my_alloc 项目

```
cargo init my_alloc
```

2、实现自定义的 my_allocator 模块

```
// src/lib/lib.rs

pub mod my_allocator {
    use std::alloc::{alloc, dealloc, Layout};
    use std::alloc::realloc as stdRealloc;

    pub fn malloc (size: usize) -> *mut u8 {
        let ptr = unsafe { alloc(Layout::from_size_align(size, std::mem::align_of::<u8>()).unwrap()) };
        ptr
    }

    pub fn free (ptr: *mut u8) {
        unsafe { dealloc(ptr, Layout::from_size_align(std::mem::size_of_val(&ptr), std::mem::align_of::<u8>()).unwrap()) };
    }

    pub fn realloc (ptr: *mut u8, size: usize) -> *mut u8 {
        let new_ptr = unsafe { stdRealloc(ptr, Layout::from_size_align(std::mem::size_of_val(&ptr), std::mem::align_of::<u8>()).unwrap(), size) };
        new_ptr
    }
}
```

3、配置库

```
// Cargo.toml

[lib]
name = "my_alloc"
path = "src/lib/lib.rs"
```

4、创建 fuzz_malloc_free target

```
cargo fuzz add fuzz_malloc_free
```

5、编写 fuzz_malloc_free 测试程序

```
// fuzz/fuzz_targets/fuzz_malloc_free.rs

#![no_main]

use libfuzzer_sys::{arbitrary::Arbitrary, fuzz_target};
use my_alloc::my_allocator;

#[derive(Arbitrary, Debug)]
enum AllocatorMethod {
    Malloc,
    Free,
    Realloc,
}

fuzz_target!(|methods: Vec<AllocatorMethod>| {
    for method in methods {
        match method {
            AllocatorMethod::Malloc => {
                let _ptr: *mut u8 = my_allocator::malloc(10);
            }
            AllocatorMethod::Free => {
                let ptr: *mut u8 = my_allocator::malloc(10);
                my_allocator::free(ptr);
            }
            AllocatorMethod::Realloc => {
                let ptr: *mut u8 = my_allocator::malloc(10);
                let _new_ptr: *mut u8 = my_allocator::realloc(ptr, 20);
            }
        }
    }
});
```

6、添加依赖

```
// fuzz/Cargo.toml

[dependencies]
arbitrary = "1.0"
libfuzzer-sys = { version = "0.4.0", features = ["arbitrary-derive"] }
```

7、运行 fuzz 测试

```
cargo fuzz run fuzz_malloc_free
```

## 参考文档

[rust fuzz book](https://rust-fuzz.github.io/book/introduction.html)
