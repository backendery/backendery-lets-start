.PHONY: all check clean install release test uninstall version

SHELL := /bin/bash

BINARY = backendery-lets-start
INSTALL_DIR = $(HOME)/.local/bin
TARGET = target/release/$(BINARY)

all: check install

check:
	@echo "🔍 Running format check and linter..."
	@cargo fmt -- --check
	@cargo clippy -- -D warnings
	@echo "✅ Code style and linter passed"
	@echo "🧪 Running tests..."
	@cargo test
	@echo "✅ All tests passed"

clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@rm -f $(INSTALL_DIR)/$(BINARY)
	@echo "🧼 Clean complete."

install: release
	@echo "📦 Installing $(BINARY)..."
	@if [ ! -d $(INSTALL_DIR) ]; then \
		echo "📁 Creating install directory $(INSTALL_DIR)..."; \
		mkdir -p $(INSTALL_DIR); \
	fi
	@cp $(TARGET) $(INSTALL_DIR)/
	@echo "✅ Installed $(BINARY) to $(INSTALL_DIR)"
	@echo "📢 Make sure $(INSTALL_DIR) is in your PATCH"

release:
	@echo "🚀 Building release binary..."
	@cargo build --release
	@echo "⚙️ Stripping debug symbols..."
	@strip $(TARGET)
	@echo "🎯 Release build ready at $(TARGET)"

test:
	@echo "🧪 Running tests..."
	@cargo test
	@echo "✅ Tests finished"

uninstall:
	@echo "🗑️ Uninstalling $(BINARY)..."
	@rm -f $(INSTALL_DIR)/$(BINARY)
	@echo "⛔ Removed $(BINARY) from $(INSTALL_DIR)"

version:
	@echo "🔖 Choose version bump type:"
	@select option in patch minor major manual; do \
		if [ -n "$$option" ]; then \
			break; \
		fi; \
	done; \
	CURRENT=$$(grep '^version =' Cargo.toml | sed 's/version = \"\(.*\)\"/\1/'); \
	IFS=. read -r MAJOR MINOR PATCH <<<"$$CURRENT"; \
	if [ "$$option" = "patch" ]; then \
		PATCH=$$((PATCH+1)); \
	elif [ "$$option" = "minor" ]; then \
		MINOR=$$((MINOR+1)); PATCH=0; \
	elif [ "$$option" = "major" ]; then \
		MAJOR=$$((MAJOR+1)); MINOR=0; PATCH=0; \
	elif [ "$$option" = "manual" ]; then \
		read -p "📝 Enter version manually (x.y.z): " INPUT; \
		if ! echo $$INPUT | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$$'; then \
			echo "⛔ Invalid version format"; \
			exit 1; \
		fi; \
		VERSION=$$INPUT; \
	else \
		exit 1; \
	fi; \
	[ -z "$$VERSION" ] && VERSION="$$MAJOR.$$MINOR.$$PATCH"; \
	if git rev-parse "v$$VERSION" >/dev/null 2>&1; then \
		echo "⛔ Tag v$$VERSION already exists"; \
		exit 1; \
	fi; \
	if [ "$$VERSION" = "$$CURRENT" ]; then \
		echo "⚠️ Version $$VERSION is same as current ($$CURRENT)"; \
		exit 1; \
	fi; \
	echo "📦 Bumping version from $$CURRENT to $$VERSION..."; \
	sed -i 's/^version = \".*\"/version = \"'$$VERSION'\"/' Cargo.toml; \
	git add .; \
	git commit -m "release: bump version to v$$VERSION 🎉"; \
	git tag -a v$$VERSION -m "Release v$$VERSION"; \
	echo "✅ Version bumped to v$$VERSION"; \
	CURRENT_BRANCH=$$(git rev-parse --abbrev-ref HEAD); \
	read -p "🚀 Push commit to GitHub? [y/N]: " CONFIRM; \
	if [ "$$CONFIRM" = "y" ] || [ "$$CONFIRM" = "Y" ]; then \
		git push origin $$CURRENT_BRANCH; \
		echo "🟢 Pushed to origin/$$CURRENT_BRANCH"; \
	else \
		echo "🛑 Push skipped. Use \`git push origin <branch name>\` manually"; \
	fi; \
	read -p "🚀 Push the latest tag to GitHub? [y/N]: " CONFIRM; \
	if [ "$$CONFIRM" = "y" ] || [ "$$CONFIRM" = "Y" ]; then \
		git push origin v$$VERSION; \
		echo "🟢 Pushed the tag v$$VERSION to origin/$$CURRENT_BRANCH"; \
	else \
		echo "🛑 Push skipped. Use \`git push origin <tag name>\` manually"; \
	fi; \
	echo "🔖 Versioning complete";