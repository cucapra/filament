# Minimal constraints file to fix hold violations
# Conservative approach - only essential timing fixes

# ========================================
# CLOCK CONSTRAINTS
# ========================================

# Primary clock constraint
create_clock -period 7.00 -name clk [get_ports clk]

# CRITICAL FIX: Force clock to use global buffer
set_property CLOCK_BUFFER_TYPE BUFG [get_nets clk]

# Out-of-context clock source constraint
set_property HD.CLK_SRC BUFGCTRL_X0Y0 [get_ports clk]

# ========================================
# INPUT/OUTPUT DELAY CONSTRAINTS - HOLD FIX
# ========================================

# MAIN FIX: Increase input delays to match clock distribution delay (~1.4ns)
# This should eliminate the majority of hold violations

# Data inputs - most critical for hold violations
set_input_delay -clock clk 2.0 [get_ports {operand_a[*] operand_b[*]}]

# Control inputs
set_input_delay -clock clk 1.5 [get_ports {operation[*] valid_in ready_in}]

# Reset - most critical path for hold
set_input_delay -clock clk 2.5 [get_ports reset]

# Output delays - keep original timing
set_output_delay -clock clk 1.0 [get_ports -filter {DIRECTION == OUT}]

# ========================================
# PHYSICAL PIN ASSIGNMENTS (Required for OOC)
# ========================================

# Set HD.PARTPIN_LOCS for input ports (except clock)
set all_input_ports [get_ports -filter {DIRECTION == IN}]
set pin_index 1
foreach port $all_input_ports {
    if {[get_property NAME $port] != "clk"} {
        set port_name [get_property NAME $port]
        set_property HD.PARTPIN_LOCS "PACKAGE_PIN_DUMMY_$pin_index" $port
        incr pin_index
    }
}

# Set HD.PARTPIN_LOCS for output ports
set all_output_ports [get_ports -filter {DIRECTION == OUT}]
foreach port $all_output_ports {
    set port_name [get_property NAME $port]
    set_property HD.PARTPIN_LOCS "PACKAGE_PIN_DUMMY_$pin_index" $port
    incr pin_index
}

puts "INFO: Applied minimal hold violation fixes - input delays increased to 1.5-2.5ns"