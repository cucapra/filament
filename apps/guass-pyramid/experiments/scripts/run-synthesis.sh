#!/bin/bash

set -eu -o pipefail


# Command pattern:
# fud e --to synth-files --from synth-verilog <file> \
#   -s synth-verilog.tcl ../synth/synth.tcl \
#   -s synth-verilog.constraints ../synth/device.xdc -vv \
#   -o <file>-out

# Global variables for job tracking
declare -a job_pids=()
declare -a job_files=()

# Signal handler to cleanup background jobs
cleanup() {
    echo "Received signal, cleaning up background jobs..."
    for pid in "${job_pids[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null || true
        fi
    done
    echo "Cleanup complete. Log files preserved."
    exit 130
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

# Check if output directory and files are provided
if [ $# -lt 2 ]; then
    echo "Error: Output directory and at least one file required. Usage: $0 <output_dir> <file1> [file2] ..." >&2
    exit 1
fi

# Extract output directory and shift arguments
output_dir="$1"
shift

# Create output directory and monitor subdirectory if they don't exist
mkdir -p "$output_dir/_monitor"

# Validate all files exist before starting any jobs
echo "Validating input files..."
for file in "$@"; do
    if [ ! -f "$file" ]; then
        echo "Error: File '$file' does not exist" >&2
        exit 1
    fi
done
echo "All files validated successfully."

# Start jobs in parallel
echo "Starting parallel synthesis jobs..."
for file in "$@"; do
    base=$(basename -s .sv $file)
    out_dir="$output_dir/$base-out"
    logfile="$output_dir/_monitor/$base.runlog"
    echo "Starting job for: $file"
    echo "Output directory: $out_dir"

    # Execute the fud command in background
    fud e --to synth-files --from synth-verilog "$file" \
        -s synth-verilog.tcl ./synth/synth.tcl \
        -s synth-verilog.constraints ./synth/device.xdc -vv \
        -o $out_dir > "$logfile" 2>&1 &

    # Store job info
    job_pids+=($!)
    job_files+=("$file")
done

total_jobs=${#job_pids[@]}
echo "Started $total_jobs jobs. Waiting for completion..."

# Wait for jobs to complete and report status
completed=0
total=${#job_pids[@]}

while [ $completed -lt $total ]; do
    for i in "${!job_pids[@]}"; do
        pid="${job_pids[$i]}"
        file="${job_files[$i]}"

        # Check if this job is still running
        if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then
            # Job completed
            wait "$pid" 2>/dev/null || true
            echo "$file completed"
            job_pids[$i]=""
            ((completed++))
        fi
    done

    # Small delay to avoid busy waiting
    sleep 0.1
done

echo "All jobs completed successfully."
