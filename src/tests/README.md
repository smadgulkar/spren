# Testing Infrastructure

## Overview
The testing infrastructure consists of several components:
- Unit tests for individual modules
- Integration tests for end-to-end functionality
- Performance benchmarks
- Error handling tests
- Test utilities for creating test environments

## Running Tests
1. Run all tests:
   ```bash
   cargo test
   ```

2. Run specific test categories:
   ```bash
   cargo test --lib  # Unit tests
   cargo test --test integration  # Integration tests
   cargo test --test performance  # Performance tests
   ```

3. Run with test coverage:
   ```bash
   cargo tarpaulin
   ```

## Test Organization
- `tests/mod.rs`: Test utilities and helpers
- `tests/integration.rs`: End-to-end integration tests
- `tests/error_handling.rs`: Error case testing
- `tests/performance.rs`: Performance benchmarks
- `tests/telemetry.rs`: Telemetry system tests
- `tests/validation.rs`: Command validation tests

## Adding New Tests
1. Unit tests should be added in the same file as the code they're testing
2. Integration tests should be added to the appropriate test file
3. Use the TestUtils helper for creating test environments
4. Follow the existing patterns for error handling and assertions 