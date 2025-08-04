# Makefile for Latency-Insensitive Interface Experiment

# Default simulator
SIM ?= iverilog
SIMFLAGS = -g2012

# Source files
PIPELINE_SOURCES = fp_adder_1stage.sv fp_adder_2stage.sv fp_adder_3stage.sv fp_adder_4stage.sv \
                   fp_mult_1stage.sv fp_mult_2stage.sv fp_mult_3stage.sv fp_mult_4stage.sv
WRAPPER_SOURCES = fp_adder_wrapper.sv fp_mult_wrapper.sv fp_adder_li_wrapper.sv fp_mult_li_wrapper.sv
ALL_SOURCES = $(PIPELINE_SOURCES) $(WRAPPER_SOURCES)

.PHONY: all clean test-variants test-static test-dynamic lint

all: test-variants test-static test-dynamic

# Test pipeline variants for consistency
test-variants: test_variants
	@echo "=== Testing Pipeline Variants ==="
	./test_variants

test_variants: test_fp_variants.sv $(PIPELINE_SOURCES)
	$(SIM) $(SIMFLAGS) -o $@ $^

# Test static ALU
test-static: test_static  
	@echo "=== Testing Static ALU ==="
	./test_static

test_static: static/tests/test_static_alu.sv static/main.sv $(ALL_SOURCES)
	$(SIM) $(SIMFLAGS) -o $@ $^

# Test dynamic ALU
test-dynamic: test_dynamic
	@echo "=== Testing Dynamic ALU ==="
	./test_dynamic

test_dynamic: dynamic/tests/test_dynamic_alu.sv dynamic/main.sv $(ALL_SOURCES)
	$(SIM) $(SIMFLAGS) -o $@ $^

# Test fixed dynamic ALU implementation
test-dynamic-fixed: test_dynamic_fixed
	@echo "=== Testing Fixed Dynamic ALU ==="
	./test_dynamic_fixed

test_dynamic_fixed: dynamic/tests/test_dynamic_alu_fixed.sv dynamic/main.sv $(ALL_SOURCES)
	$(SIM) $(SIMFLAGS) -o $@ $^

# Lint all modules
lint:
	@echo "=== Linting All Modules ==="
	@for file in $(ALL_SOURCES) static/main.sv dynamic/main.sv; do \
		echo "Linting $$file..."; \
		verilator --lint-only $$file || true; \
	done

# Quick syntax check
syntax-check:
	@echo "=== Syntax Check ==="
	@for file in $(ALL_SOURCES) static/main.sv dynamic/main.sv; do \
		echo "Checking $$file..."; \
		$(SIM) -t null $$file || true; \
	done

# Run all tests
test: test-variants test-static test-dynamic-fixed
	@echo "=== All Tests Completed ==="

# Generic rule to compile and run any Verilog testbench
# Usage: make run TESTBENCH=my_test.sv [SOURCES="file1.sv file2.sv"] [OUTPUT=my_test]
# Example: make run TESTBENCH=debug_dynamic_alu.sv SOURCES="dynamic/main.sv fp_*.sv"
# Example: make run TESTBENCH=simple_test.sv OUTPUT=simple
run:
	@if [ -z "$(TESTBENCH)" ]; then \
		echo "Error: TESTBENCH parameter is required"; \
		echo "Usage: make run TESTBENCH=testbench.sv [SOURCES=\"src1.sv src2.sv\"] [OUTPUT=output_name]"; \
		echo "Example: make run TESTBENCH=debug_test.sv SOURCES=\"dynamic/main.sv fp_*.sv\""; \
		exit 1; \
	fi
	@OUTPUT_NAME=$${OUTPUT:-$$(basename $(TESTBENCH) .sv)}; \
	SOURCE_FILES="$${SOURCES:-$(ALL_SOURCES)}"; \
	echo "=== Compiling and Running $$OUTPUT_NAME ==="; \
	echo "Testbench: $(TESTBENCH)"; \
	echo "Sources: $$SOURCE_FILES"; \
	$(SIM) $(SIMFLAGS) -o $$OUTPUT_NAME $(TESTBENCH) $$SOURCE_FILES && \
	echo "=== Executing $$OUTPUT_NAME ===" && \
	./$$OUTPUT_NAME

# Clean generated files
clean:
	rm -f test_variants test_static test_dynamic test_dynamic_fixed
	rm -f debug_test debug_bp
	rm -f *.vcd *.dmp
	rm -f a.out

# Help
help:
	@echo "Available targets:"
	@echo "  all           - Build all test executables"
	@echo "  test          - Run all tests"
	@echo "  test-variants - Test pipeline variant consistency"
	@echo "  test-static   - Test static ALU configurations"
	@echo "  test-dynamic  - Test dynamic ALU configurations"
	@echo "  test-dynamic-fixed - Test fixed dynamic ALU implementation"
	@echo "  run           - Compile and run custom testbench (see usage below)"
	@echo "  lint          - Run Verilator lint on all modules"
	@echo "  syntax-check  - Quick syntax verification"
	@echo "  clean         - Remove generated files"
	@echo "  help          - Show this help message"
	@echo ""
	@echo "Generic testbench runner usage:"
	@echo "  make run TESTBENCH=testbench.sv [SOURCES=\"src1.sv src2.sv\"] [OUTPUT=output_name]"
	@echo ""
	@echo "Examples:"
	@echo "  make run TESTBENCH=debug_dynamic_alu.sv"
	@echo "  make run TESTBENCH=my_test.sv SOURCES=\"dynamic/main.sv fp_*.sv\""
	@echo "  make run TESTBENCH=simple_test.sv OUTPUT=simple"