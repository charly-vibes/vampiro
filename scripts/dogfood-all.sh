#!/usr/bin/env bash
# Run vampiro check on all source files. Exit non-zero if findings detected.
# Called by: just dogfood, lefthook pre-push

set -euo pipefail

cargo build --quiet 2>/dev/null
binary="./target/debug/vampiro"
total=0

for file in $(find crates -name "*.rs" -not -path "*/target/*" -not -path "*/tests/*" -not -path "*/fixtures/*" | sort); do
    output=$("$binary" check --path "$file" 2>/dev/null)
    if echo "$output" | grep -q "^crates/"; then
        echo "$output"
        count=$(echo "$output" | grep -c "^crates/" || true)
        total=$((total + count))
    fi
done

echo "⟶  Scanned $(find crates -name '*.rs' ! -path '*/target/*' ! -path '*/tests/*' ! -path '*/fixtures/*' | wc -l) files, $total finding(s)."
[ "$total" -eq 0 ]