PLATFORM ?= osx_amd64
DUCKDB_API_VERSION ?= v1.2.0

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
