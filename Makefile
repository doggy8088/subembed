# Makefile for subembed

# Variables
CARGO = cargo
TARGET = release
PREFIX ?= $(HOME)/.cargo
BINDIR = $(PREFIX)/bin
BINARY_NAME = subembed
TARGET_BIN = target/$(TARGET)/$(BINARY_NAME)

.PHONY: all build install uninstall clean test fmt lint help

# Default target
all: build ## Build the release binary (default target)

# Build the release binary
build: ## Build the release binary
	$(CARGO) build --release

# Run tests
test: ## Run tests
	$(CARGO) test

# Run formatting checks
fmt: ## Run formatting checks
	$(CARGO) fmt --all -- --check

# Run lint checks
lint: ## Run lint checks
	$(CARGO) clippy --all-targets -- -D warnings

# Install the binary to the system
install: build ## Install the binary to the system
	install -d $(DESTDIR)$(BINDIR)
	install -m 755 $(TARGET_BIN) $(DESTDIR)$(BINDIR)/$(BINARY_NAME)

# Uninstall the binary from the system
uninstall: ## Uninstall the binary from the system
	rm -f $(DESTDIR)$(BINDIR)/$(BINARY_NAME)

# Clean build artifacts
clean: ## Clean build artifacts
	$(CARGO) clean

# Show this help message
help: ## Show this help message
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2}'
