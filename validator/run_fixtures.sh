#!/bin/sh
# Fixture harness: each snapshot declares the exit code it must produce.
#   0 = all invariants hold
#   1 = at least one violated
#
# A fixture "passes" when the validator reaches the expected verdict, so a
# corrupt snapshot that quietly validates is a failure of this harness, not a
# success. POSIX sh throughout: macOS still ships bash 3.2, which has no
# associative arrays.

expected_for() {
    case "$1" in
        01_valid_open.json)               echo 0 ;;
        02_valid_resolved.json)           echo 0 ;;
        03_tampered_balance.json)         echo 1 ;;
        04_illegal_status_jump.json)      echo 1 ;;
        05_resolved_before_deadline.json) echo 1 ;;
        *)                                echo "?" ;;
    esac
}

if [ ! -x ./validate ]; then
    echo "./validate not found. Run 'make' first."
    exit 2
fi

pass=0
fail=0

for f in fixtures/*.json; do
    name=`basename "$f"`
    want=`expected_for "$name"`

    if [ "$want" = "?" ]; then
        echo "=== $name ==="
        echo ">>> SKIPPED: no expected exit code declared"
        echo
        continue
    fi

    echo "=== $name (expect exit $want) ==="
    ./validate "$f"
    got=$?

    if [ "$got" -eq "$want" ]; then
        echo ">>> fixture OK"
        pass=`expr $pass + 1`
    else
        echo ">>> FIXTURE MISMATCH: expected exit $want, got $got"
        fail=`expr $fail + 1`
    fi
    echo
done

echo "fixtures: $pass passed, $fail failed"
[ "$fail" -eq 0 ]