PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share

.PHONY: all build release install uninstall test check clean

all: release

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

check:
	cargo check

install: release
	install -d $(DESTDIR)$(BINDIR)
	install -m 755 target/release/stasis $(DESTDIR)$(BINDIR)/stasis
	install -d $(DESTDIR)$(DATADIR)/applications
	install -m 644 stasis.desktop $(DESTDIR)$(DATADIR)/applications/stasis.desktop

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/stasis
	rm -f $(DESTDIR)$(DATADIR)/applications/stasis.desktop

clean:
	cargo clean
