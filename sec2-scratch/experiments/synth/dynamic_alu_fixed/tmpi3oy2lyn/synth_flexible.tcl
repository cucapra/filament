# Flexible synthesis script that works with any .xdc file
# Run this by typing:
#
#   vivado -mode batch -source synth_flexible.tcl
#
# Then see the resource utilization (i.e., area) report dumped at:
#
#   out/FutilBuild.runs/synth_1/main_utilization_synth.rpt
#
# And if you also do implementation (see below), see the timing report:
#
#   out/FutilBuild.runs/impl_1/main_timing_summary_routed.rpt

# Settings: the output directory and the part number (which is a Zynq
# XC7Z020, found on our ZedBoard).
set outdir ./out
# set partname xc7z020clg484-1
# You can also use part name "xcu250-figd2104-2-e", which we get on havarti.
# This is a bigger device (larger memory, etc.) and also supports URAM memory, which
# "xczu3eg-sbva484-1-e" does not support. For more information on
# this part type look here: https://docs.xilinx.com/r/en-US/ds962-u200-u250/Summary
set partname "xczu3eg-sbva484-1-e"

# Create the project (forcibly overwriting) and add sources SystemVerilog
# (*.sv) and any Xilinx constraint file (*.xdc).
#
# UPDATED: Use flexible glob pattern to find any .xdc file
create_project -force -part $partname FutilBuild $outdir
add_files [glob ./*.sv]

# Find and add any .xdc constraint files
set xdc_files [glob -nocomplain ./*.xdc]
if {[llength $xdc_files] > 0} {
    add_files -fileset constrs_1 $xdc_files
    puts "INFO: Added constraint files: $xdc_files"
} else {
    puts "WARNING: No .xdc constraint files found"
}

set_property top main [current_fileset]

# Switch the project to "out-of-context" mode, which frees us from the need to
# hook up every input & output wire to a physical device pin.
set_property \
    -name {STEPS.SYNTH_DESIGN.ARGS.MORE OPTIONS} \
    -value {-mode out_of_context -flatten_hierarchy "rebuilt"} \
    -objects [get_runs synth_1]

# Run synthesis. This is enough to generate the utilization report mentioned
# above but does not include timing information.
launch_runs synth_1
wait_on_run synth_1

# Run implementation to do place & route. This also produces the timing
# report mentioned above. Removing this step makes things go quite a bit
# faster if you just need the resource report!
launch_runs impl_1 -to_step route_design
wait_on_run impl_1

# Obtain hierarchical report on resource utilization
open_run impl_1
report_utilization -hierarchical -file $outdir/hierarchical_utilization_placed.rpt
report_timing -file $outdir/main_timing_report.rpt
close_design