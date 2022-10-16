set outdir ./out
set partname "xczu3eg-sbva484-1-e"
set IP "./IP"
file mkdir $IP

# Generate integer multiplier
create_project -in_memory -part $partname
set moduleName mul_uint8
set bitWidth 8
set outHighBit 15
set pipeStages 3
create_ip -name mult_gen -vendor xilinx.com -library ip -version 12.0 -module_name $moduleName -dir $IP
set_property -dict [list CONFIG.PortAType {Unsigned} CONFIG.PortAWidth "$bitWidth" CONFIG.PortBType {Unsigned} CONFIG.PortBWidth "$bitWidth" CONFIG.Multiplier_Construction {Use_Mults}  CONFIG.Use_Custom_Output_Width {true} CONFIG.OutputWidthHigh "$outHighBit" CONFIG.PipeStages "$pipeStages" CONFIG.ClockEnable {false}] [get_ips $moduleName]
generate_target {all} [get_ips]
synth_ip [get_ips]

# Create a new project and read IP
create_project -force -part $partname FutilBuild $outdir
read_ip [glob "$IP/*/*.xci"]
add_files -quiet [glob -nocomplain ./*.sv]
add_files -fileset constrs_1 [glob ./*.xdc]
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
