#!/bin/bash

echo "Running unit tests..."
cargo test --lib || exit 1

echo "Running integration tests..."
cargo test --test integration || exit 1

echo "Running test coverage analysis..."
cargo tarpaulin --out Html || exit 1

echo "Running specific component tests..."
cargo test --package spren --lib telemetry || exit 1
cargo test --package spren --lib validation || exit 1

echo "All tests completed successfully!" 