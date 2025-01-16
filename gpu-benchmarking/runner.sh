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
ITERATIONS=50
CPU_SUFFIX="_cpu"
GPU_SUFFIX="_gpu"
CPU_BINARY="../target/release/ezkl${CPU_SUFFIX}"
GPU_BINARY="../target/release/ezkl${GPU_SUFFIX}"
CPU_SETUP_CMD="$CPU_BINARY setup"
GPU_SETUP_CMD="$GPU_BINARY setup"
CPU_PROVE_CMD="$CPU_BINARY prove"
GPU_PROVE_CMD="$GPU_BINARY prove"

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
cpu_setup_total_time=0
gpu_setup_total_time=0
cpu_prove_total_time=0
gpu_prove_total_time=0

# Benchmark GPU setup
echo "Running GPU setup $ITERATIONS times..."
for i in $(seq 1 $ITERATIONS); do
    start_time=$(date +%s.%N)
    $GPU_SETUP_CMD > /dev/null 2>&1
    end_time=$(date +%s.%N)
    elapsed_time=$(echo "$end_time - $start_time" | bc)
    gpu_setup_total_time=$(echo "$gpu_setup_total_time + $elapsed_time" | bc)
    echo "GPU Setup Run #$i: $elapsed_time seconds"
done

# Benchmark CPU setup
echo "Running CPU setup $ITERATIONS times..."
for i in $(seq 1 $ITERATIONS); do
    start_time=$(date +%s.%N)
    $CPU_SETUP_CMD > /dev/null 2>&1
    end_time=$(date +%s.%N)
    elapsed_time=$(echo "$end_time - $start_time" | bc)
    cpu_setup_total_time=$(echo "$cpu_setup_total_time + $elapsed_time" | bc)
    echo "CPU Setup Run #$i: $elapsed_time seconds"
done

# Benchmark GPU prove
echo "Running GPU prove $ITERATIONS times..."
for i in $(seq 1 $ITERATIONS); do
    start_time=$(date +%s.%N)
    $GPU_PROVE_CMD > /dev/null 2>&1
    end_time=$(date +%s.%N)
    elapsed_time=$(echo "$end_time - $start_time" | bc)
    gpu_prove_total_time=$(echo "$gpu_prove_total_time + $elapsed_time" | bc)
    echo "GPU Prove Run #$i: $elapsed_time seconds"
done

# Benchmark CPU prove
echo "Running CPU prove $ITERATIONS times..."
for i in $(seq 1 $ITERATIONS); do
    start_time=$(date +%s.%N)
    $CPU_PROVE_CMD > /dev/null 2>&1
    end_time=$(date +%s.%N)
    elapsed_time=$(echo "$end_time - $start_time" | bc)
    cpu_prove_total_time=$(echo "$cpu_prove_total_time + $elapsed_time" | bc)
    echo "CPU Prove Run #$i: $elapsed_time seconds"
done

# Calculate averages
cpu_setup_avg_time=$(echo "$cpu_setup_total_time / $ITERATIONS" | bc -l)
gpu_setup_avg_time=$(echo "$gpu_setup_total_time / $ITERATIONS" | bc -l)
cpu_prove_avg_time=$(echo "$cpu_prove_total_time / $ITERATIONS" | bc -l)
gpu_prove_avg_time=$(echo "$gpu_prove_total_time / $ITERATIONS" | bc -l)

# Output results
echo "======================================="
echo "Setup Results:"
echo "CPU Average Setup Time: $cpu_setup_avg_time seconds"
echo "GPU Average Setup Time: $gpu_setup_avg_time seconds"
echo "---------------------------------------"
echo "Prove Results:"
echo "CPU Average Prove Time: $cpu_prove_avg_time seconds"
echo "GPU Average Prove Time: $gpu_prove_avg_time seconds"
echo "======================================="