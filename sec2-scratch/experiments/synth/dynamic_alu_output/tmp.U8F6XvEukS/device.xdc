create_clock -period 7.00 -name clk [get_ports clk]

# Out-of-context constraints to reduce warnings and enable better timing analysis
# Set HD.CLK_SRC property for the clock port
set_property HD.CLK_SRC BUFGCTRL_X0Y0 [get_ports clk]

# Automatically set HD.PARTPIN_LOCS for all input ports (except clock)
# This prevents the routing warnings in out-of-context mode
set all_input_ports [get_ports -filter {DIRECTION == IN}]
set pin_index 1
puts "INFO: Setting HD.PARTPIN_LOCS for [llength $all_input_ports] input ports"
foreach port $all_input_ports {
    if {[get_property NAME $port] != "clk"} {
        set port_name [get_property NAME $port]
        set_property HD.PARTPIN_LOCS "PACKAGE_PIN_DUMMY_$pin_index" $port
        puts "  - Set $port_name to PACKAGE_PIN_DUMMY_$pin_index"
        incr pin_index
    }
}

# Also set HD.PARTPIN_LOCS for all output ports
set all_output_ports [get_ports -filter {DIRECTION == OUT}]
puts "INFO: Setting HD.PARTPIN_LOCS for [llength $all_output_ports] output ports"
foreach port $all_output_ports {
    set port_name [get_property NAME $port]
    set_property HD.PARTPIN_LOCS "PACKAGE_PIN_DUMMY_$pin_index" $port
    puts "  - Set $port_name to PACKAGE_PIN_DUMMY_$pin_index"
    incr pin_index
}

# Set input delays for timing analysis (1ns delay on all input ports except clock)
set_input_delay -clock clk 0.05 [get_ports -filter {DIRECTION == IN && NAME != "clk"}]

# Set output delays for timing analysis (1ns delay on all output ports)
set_output_delay -clock clk 0.05 [get_ports -filter {DIRECTION == OUT}]
