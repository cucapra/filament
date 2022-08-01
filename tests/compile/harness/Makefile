# defaults
SIM ?= icarus
TOPLEVEL_LANG ?= verilog

VERILOG_SOURCES += $(PWD)/out.sv
# use VHDL_SOURCES for VHDL files

# TOPLEVEL is the name of the toplevel module in your Verilog or VHDL file
TOPLEVEL = main

# MODULE is the basename of the Python test file
MODULE = sim

ifeq ($(FIL),)
  $(error "Provide the input filament file by setting FIL")
endif

# include cocotb's make rules to take care of the simulator setup
include $(shell cocotb-config --makefiles)/Makefile.sim

.INTERMEDIATE: $(PWD)/out.sv

$(PWD)/out.sv: $(FIL)
	fud e $< -o $@ -s futil.flags ' -d canonicalize' --to icarus-verilog -q
