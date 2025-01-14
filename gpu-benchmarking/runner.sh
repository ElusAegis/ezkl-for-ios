#!/bin/bash

# Ensure the script is being run from the correct directory
EXPECTED_DIR="gpu-benchmarking"
if [ "$(basename "$PWD")" != "$EXPECTED_DIR" ]; then
    echo "You are not in the '$EXPECTED_DIR' directory. Attempting to cd into it..."
    if [ -d "$EXPECTED_DIR" ]; then
        cd "$EXPECTED_DIR" || { echo "Failed to cd into $EXPECTED_DIR. Exiting."; exit 1; }
    else
        echo "Directory '$EXPECTED_DIR' not found. Exiting."
        exit 1
    fi
fi


# Define variables
ITERATIONS=30
CPU_SUFFIX="_cpu"
GPU_SUFFIX="_gpu_2_5"
CPU_BINARY="../target/release/ezkl${CPU_SUFFIX}"
GPU_BINARY="../target/release/ezkl${GPU_SUFFIX}"
CPU_CMD="$CPU_BINARY prove"
GPU_CMD="$GPU_BINARY prove"

# Build binaries
echo "Building CPU binary..."
cargo build --release
mv ../target/release/ezkl $CPU_BINARY

echo "Building GPU binary..."
cargo build --release --features "metal"
mv ../target/release/ezkl $GPU_BINARY

# Prepare necessary files
echo "Generating test files..."
$CPU_BINARY compile-circuit > /dev/null 2>&1
$CPU_BINARY setup > /dev/null 2>&1
$CPU_BINARY gen-witness > /dev/null 2>&1

# Initialize totals
cpu_total_time=0
gpu_total_time=0

# Run CPU benchmark
echo "Running $CPU_CMD $ITERATIONS times..."
for i in $(seq 1 $ITERATIONS); do
    start_time=$(date +%s.%N)
    $CPU_CMD > /dev/null 2>&1
    end_time=$(date +%s.%N)
    elapsed_time=$(echo "$end_time - $start_time" | bc)
    cpu_total_time=$(echo "$cpu_total_time + $elapsed_time" | bc)
    echo "CPU Run #$i: $elapsed_time seconds"
done

# Run GPU benchmark
echo "Running $GPU_CMD $ITERATIONS times..."
for i in $(seq 1 $ITERATIONS); do
    start_time=$(date +%s.%N)
    $GPU_CMD > /dev/null 2>&1
    end_time=$(date +%s.%N)
    elapsed_time=$(echo "$end_time - $start_time" | bc)
    gpu_total_time=$(echo "$gpu_total_time + $elapsed_time" | bc)
    echo "GPU Run #$i: $elapsed_time seconds"
done

# Calculate averages
cpu_avg_time=$(echo "$cpu_total_time / $ITERATIONS" | bc -l)
gpu_avg_time=$(echo "$gpu_total_time / $ITERATIONS" | bc -l)

# Output results
echo "======================================="
echo "CPU Average Time: $cpu_avg_time seconds"
echo "GPU Average Time: $gpu_avg_time seconds"
echo "======================================="