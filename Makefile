.PHONY: test
test:
	python3 -m unittest
	$(MAKE) test-sdist
	cargo build --bin asnkitcc
	PATH=$$PWD/target/debug:$$PATH uv run python examples/compact_extensions_uper/main.py
	PATH=$$PWD/target/debug:$$PATH uv run python examples/programming_types/main.py

.PHONY: lint
lint:
	uv run ruff check
	uv run ruff format --check

.PHONY: codespell
codespell:
	codespell -d $$(git ls-files | grep -v ietf \
				     | grep -v 3gpp \
				     | grep -v generated \
				     | grep -v "\.asn")

.PHONY: test-sdist
test-sdist:
	rm -rf dist
	uv build --sdist --all-packages
	cd dist && \
	mkdir test && \
	cd test && \
	tar xf ../asnkitrt-[0-9]*.tar.gz && \
	tar xf ../asnkit_gen_python-*.tar.gz && \
	env PYTHONPATH=$$(echo asnkitrt-[0-9]*/src):$$(echo asnkit_gen_python-*/src) \
	    python3 -c "import asnkitrt, asnkit_gen_python"

.PHONY: release-to-pypi
release-to-pypi:
	rm -rf dist
	uv build --all-packages
	twine upload dist/*

.PHONY: docs
docs:
	uv run --group docs mkdocs build --strict

.PHONY: docs-serve
docs-serve:
	uv run --group docs mkdocs serve
