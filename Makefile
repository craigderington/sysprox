.PHONY: build run clean install test fmt clippy check help release dev

BINARY_NAME=sysprox
VERSION?=0.2.0
INSTALL_DIR=/usr/local/bin
TARGET_DIR=target

# Build the application (debug)
build:
	@echo "Building $(BINARY_NAME) (debug)..."
	@cargo build

# Build release version
release:
	@echo "Building $(BINARY_NAME) (release)..."
	@cargo build --release

# Run the application
run:
	@echo "Running $(BINARY_NAME)..."
	@cargo run

# Run in release mode
run-release:
	@echo "Running $(BINARY_NAME) (release)..."
	@cargo run --release

# Clean build artifacts
clean:
	@echo "Cleaning..."
	@cargo clean
	@rm -f Cargo.lock

# Install the binary to system (release build)
install: release
	@echo "Installing $(BINARY_NAME) to $(INSTALL_DIR)..."
	@sudo install -m 755 $(TARGET_DIR)/release/$(BINARY_NAME) $(INSTALL_DIR)/

# Uninstall the binary from system
uninstall:
	@echo "Uninstalling $(BINARY_NAME)..."
	@sudo rm -f $(INSTALL_DIR)/$(BINARY_NAME)

# Run tests
test:
	@echo "Running tests..."
	@cargo test

# Run tests with output
test-verbose:
	@echo "Running tests (verbose)..."
	@cargo test -- --nocapture

# Run tests with coverage (requires cargo-tarpaulin)
test-coverage:
	@echo "Running tests with coverage..."
	@cargo tarpaulin --out Html --output-dir coverage
	@echo "Coverage report generated: coverage/index.html"

# Format code
fmt:
	@echo "Formatting code..."
	@cargo fmt

# Check formatting
fmt-check:
	@echo "Checking code format..."
	@cargo fmt -- --check

# Run clippy (linter)
clippy:
	@echo "Running clippy..."
	@cargo clippy -- -D warnings

# Check code without building
check:
	@echo "Checking code..."
	@cargo check

# Check all targets and features
check-all:
	@echo "Checking all targets..."
	@cargo check --all-targets --all-features

# Update dependencies
update-deps:
	@echo "Updating dependencies..."
	@cargo update

# Audit dependencies for security vulnerabilities
audit:
	@echo "Auditing dependencies..."
	@cargo audit

# Build documentation
doc:
	@echo "Building documentation..."
	@cargo doc --no-deps --open

# Development build with all checks
dev: fmt clippy test
	@echo "Development checks complete!"

# CI checks (format, clippy, test)
ci: fmt-check clippy test
	@echo "CI checks passed!"

# Show cargo tree (dependency tree)
tree:
	@cargo tree

# Show outdated dependencies
outdated:
	@cargo outdated

# Benchmark (if benchmarks exist)
bench:
	@echo "Running benchmarks..."
	@cargo bench

# Build for release with all optimizations
release-optimized: clean
	@echo "Building optimized release..."
	@RUSTFLAGS="-C target-cpu=native" cargo build --release

# Create distributable package
package: release
	@echo "Creating package..."
	@mkdir -p dist
	@cp $(TARGET_DIR)/release/$(BINARY_NAME) dist/
	@tar -czf dist/$(BINARY_NAME)-$(VERSION)-linux-x86_64.tar.gz -C dist $(BINARY_NAME)
	@echo "Package created: dist/$(BINARY_NAME)-$(VERSION)-linux-x86_64.tar.gz"

# Show help
help:
	@echo "Sysprox Makefile Commands:"
	@echo ""
	@echo "Build:"
	@echo "  make build             - Build debug version"
	@echo "  make release           - Build release version"
	@echo "  make release-optimized - Build with native CPU optimizations"
	@echo ""
	@echo "Run:"
	@echo "  make run               - Run debug version"
	@echo "  make run-release       - Run release version"
	@echo ""
	@echo "Development:"
	@echo "  make dev               - Run fmt, clippy, and tests"
	@echo "  make fmt               - Format code"
	@echo "  make fmt-check         - Check code format"
	@echo "  make clippy            - Run clippy linter"
	@echo "  make check             - Check code without building"
	@echo ""
	@echo "Testing:"
	@echo "  make test              - Run tests"
	@echo "  make test-verbose      - Run tests with output"
	@echo "  make test-coverage     - Generate coverage report"
	@echo "  make bench             - Run benchmarks"
	@echo ""
	@echo "Installation:"
	@echo "  make install           - Install to $(INSTALL_DIR)"
	@echo "  make uninstall         - Remove from $(INSTALL_DIR)"
	@echo "  make package           - Create distributable package"
	@echo ""
	@echo "Maintenance:"
	@echo "  make clean             - Remove build artifacts"
	@echo "  make update-deps       - Update dependencies"
	@echo "  make audit             - Check for security vulnerabilities"
	@echo "  make doc               - Build and open documentation"
	@echo "  make tree              - Show dependency tree"
	@echo "  make outdated          - Show outdated dependencies"
	@echo ""
	@echo "CI:"
	@echo "  make ci                - Run all CI checks"
	@echo ""
	@echo "  make help              - Show this help message"
