DUCKDB_API_VERSION ?= v1.2.0

# Auto-detect DuckDB platform string from OS + arch
OS   := $(shell uname -s | tr '[:upper:]' '[:lower:]')
ARCH := $(shell uname -m)

ifeq ($(OS),darwin)
  ifeq ($(ARCH),arm64)
    PLATFORM ?= osx_arm64
  else
    PLATFORM ?= osx_amd64
  endif
else ifeq ($(OS),linux)
  ifeq ($(ARCH),aarch64)
    PLATFORM ?= linux_arm64
  else
    PLATFORM ?= linux_amd64
  endif
else
  $(error Unsupported platform: $(OS)/$(ARCH))
endif

.PHONY: build package clean

build:
	cargo build --release

package: build
	append_metadata \
		target/release/libjev.dylib \
		jev.duckdb_extension \
		--duckdb-version $(DUCKDB_API_VERSION) \
		--platform $(PLATFORM) \
		--replace \
		--dump

clean:
	cargo clean
	rm -f jev.duckdb_extension
