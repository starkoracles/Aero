BIN_DIR = ./bin
STARK_PARSER = $(BIN_DIR)/stark_parser

$(STARK_PARSER): $(addprefix miden-to-cairo-parser/src/,lib.rs main.rs memory.rs)
	cargo build;
	mkdir -p bin;
	cp target/debug/miden_to_cairo_parser bin/stark_parser

generate_proof: 
	cargo run -p miden_proof_generator

integration_test: $(STARK_PARSER)
	@echo "Running integration tests..."
	PYTHONPATH=$$(echo pwd)/tests:$$(python -c "import site; print(site.getsitepackages()[0])"):$$PYTHONPATH protostar -p integration test --max-steps 100000000