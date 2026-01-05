#!/bin/bash
# Wrapper script to ensure rustup's cargo is used instead of Homebrew's

# Get the rustup cargo path
RUSTUP_CARGO=$(rustup which cargo 2>/dev/null)

if [ -n "$RUSTUP_CARGO" ] && [ -x "$RUSTUP_CARGO" ]; then
    # Use rustup's cargo
    exec "$RUSTUP_CARGO" "$@"
else
    # Fallback to system cargo
    exec cargo "$@"
fi

