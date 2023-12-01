SRCS = $(wildcard *.c)
PROGS = $(patsubst %.c,%,$(SRCS))
PROGS_LN_RUST = $(patsubst %.c,%_rust,$(SRCS))

RELEASE_DIR = $(patsubst %/exts/libudev/tests,%/target/release/,$(CURDIR))

CFLAGS=-lpthread -L/lib64 -ludev

CFLGAS_LN_RUST=-lpthread -L $(RELEASE_DIR) -ludev

all: c rust

c: $(PROGS)

%: %.c
	$(CC) $(CFLAGS) -o $@ $<

rust: build_release $(PROGS_LN_RUST)

build_release:
	cargo build -p libudev --release

%_rust: %.c
	$(CC) $(CFLGAS_LN_RUST) -o $@ $<

clean:
	rm -f $(PROGS)
	rm -f $(PROGS_LN_RUST)
