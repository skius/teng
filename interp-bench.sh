#!/bin/bash
# This script takes bench.csv, groups by benchmark name, and computes
# the %-delta between the average frame counts for each git commit hash.
# It prints a sequential list per benchmark.
#
# The CSV file should have the following columns:
#   git hash,unix time,bench recording name,frames

CSV_FILE="bench.csv"
if [[ ! -f "$CSV_FILE" ]]; then
    echo "Error: $CSV_FILE not found!"
    exit 1
fi

# Declare associative arrays to hold:
#   - the sum of frames per bench+commit key,
#   - the count of entries per bench+commit,
#   - and the earliest timestamp for that bench+commit.
declare -A sum
declare -A count
declare -A first_time

# Skip header and read the CSV
read -r header < "$CSV_FILE"
while IFS=, read -r git_hash unix_time bench frames; do
    # Skip empty lines (or if git_hash is empty)
    [[ -z "$git_hash" ]] && continue

    key="${bench}|${git_hash}"
    sum["$key"]=$(( ${sum["$key"]:-0} + frames ))
    count["$key"]=$(( ${count["$key"]:-0} + 1 ))
    # Record the earliest timestamp (in case multiple lines exist for a given key)
    if [[ -z "${first_time[$key]}" ]] || (( unix_time < first_time[$key] )); then
        first_time["$key"]="$unix_time"
    fi
done < <(tail -n +2 "$CSV_FILE")  # tail skips the header

# Compute the average frame count for each bench+commit key.
declare -A avg
for key in "${!sum[@]}"; do
    avg["$key"]=$(echo "scale=2; ${sum[$key]} / ${count[$key]}" | bc)
done

# Group commit hashes per benchmark.
declare -A bench_commits
for key in "${!sum[@]}"; do
    bench="${key%%|*}"
    hash="${key##*|}"
    # Append hash and its timestamp (separated by a comma) to the benchmark entry.
    bench_commits["$bench"]+="${hash},${first_time[$key]} "
done

# Output the results in the desired format.
for bench in "${!bench_commits[@]}"; do
    echo "${bench}:"
    # For each benchmark, sort the commit hash entries by timestamp.
    sorted_commits=$(echo "${bench_commits[$bench]}" | tr ' ' '\n' | grep -v '^$' | sort -t, -k2,2n)

    # Read the sorted commits into an array.
    commit_array=()
    while IFS=, read -r hash time; do
        commit_array+=("$hash")
    done <<< "$sorted_commits"

    # Iterate over the commits, compute delta vs. previous commit.
    prev_avg=""
    for i in "${!commit_array[@]}"; do
        hash="${commit_array[i]}"
        key="${bench}|${hash}"
        current_avg="${avg[$key]}"
        if [[ $i -eq 0 ]]; then
            # For the first commit, delta is defined as +0%
            delta="+0%"
        else
            # Compute percentage change relative to previous average:
            #   delta = 100 * (current_avg - prev_avg) / prev_avg
            delta_calc=$(echo "scale=2; 100*(${current_avg} - ${prev_avg})/${prev_avg}" | bc)
            # Prepend a '+' if the delta is positive or zero
            if [[ $delta_calc =~ ^- ]]; then
                delta="${delta_calc}%"
            else
                delta="+${delta_calc}%"
            fi
        fi
        echo "  - ${hash}, ${current_avg}, ${delta}"
        prev_avg="${current_avg}"
    done
    echo    # add a blank line between benchmarks
done