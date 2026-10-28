#!/bin/sh
# Generate expected output for execution tests with Free Pascal in Turbo
# Pascal mode: compile each given .pas file with `fpc -Mtp`, run it (with
# NAME.in as stdin if present), and save stdout as NAME.out.
#
# Usage: tests/gen_expected.sh tests/run/foo.pas [more.pas ...]
set -eu
build=$(mktemp -d)
trap 'rm -rf "$build"' EXIT
for src in "$@"; do
    name=$(basename "$src" .pas)
    dir=$(dirname "$src")
    if ! fpc -Mtp -v0 -FE"$build" -o"$build/$name" "$src" > "$build/fpc.log" 2>&1; then
        grep -v "macosx_version_min\|multiply_defined" "$build/fpc.log" >&2
        echo "$src: fpc failed" >&2
        exit 1
    fi
    input="$dir/$name.in"
    [ -f "$input" ] || input=/dev/null
    set +e
    "$build/$name" < "$input" > "$dir/$name.out"
    code=$?
    set -e
    if [ "$code" -ne 0 ]; then
        echo "$code" > "$dir/$name.exitcode"
    fi
    echo "$src -> $dir/$name.out (exit $code)"
done
