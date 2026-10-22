#!/usr/bin/env bash
# End-to-end run of the real binary against a local fixture server (Tier 3: not part of
# scripts/check). Usage: scripts/e2e/run.sh [path/to/rget]
set -uo pipefail

here=$(cd "$(dirname "$0")" && pwd)
bin=${1:-"$here/../../target/release/rget"}
[ -x "$bin" ] || { echo "no binary at $bin (cargo build --release)"; exit 2; }

work=$(mktemp -d)
port=$((20000 + RANDOM % 20000))
mkdir "$work/www" "$work/out"
head -c 300000 /dev/urandom > "$work/www/data.bin"
printf 'hello\n' > "$work/www/hello.txt"
python3 "$here/server.py" "$work/www" "$port" &
server=$!
trap 'kill $server 2>/dev/null; rm -rf "$work"' EXIT
for _ in $(seq 50); do (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null && break; sleep 0.1; done

base="http://127.0.0.1:$port"
want=$(sha256sum "$work/www/data.bin" | cut -d' ' -f1)
pass=0
fail=0
cd "$work/out" || exit 2
r() { "$bin" --no-config --allow-private "$@"; }

# expect <name> <expected-exit-code> <command...>
expect() {
    local name=$1 code=$2
    shift 2
    "$@" >/dev/null 2>&1
    local got=$?
    if [ "$got" -eq "$code" ]; then pass=$((pass + 1)); else fail=$((fail + 1)); echo "FAIL $name (exit $got, wanted $code)"; fi
}
same() { cmp -s "$1" "$2"; }

expect "plain download" 0 r -q "$base/hello.txt"
expect "saved to cwd" 0 test -f hello.txt
expect "-O names the file" 0 r -q -O renamed.bin "$base/data.bin"
expect "content identical" 0 same renamed.bin "$work/www/data.bin"
expect "sha256 match" 0 r -q -O ok.bin --sha256 "$want" "$base/data.bin"
expect "sha256 mismatch fails" 1 r -q -O bad.bin --sha256 "$(printf '0%.0s' $(seq 64))" "$base/data.bin"
expect "mismatch leaves nothing" 0 test ! -e bad.bin
expect "segments identical" 0 bash -c "'$bin' --no-config --allow-private -q --segments 4 -O seg.bin '$base/data.bin' && cmp -s seg.bin '$work/www/data.bin'"
expect "skip keeps file" 0 bash -c "'$bin' --no-config --allow-private -q --if-exists skip -O ok.bin '$base/hello.txt'; cmp -s ok.bin '$work/www/data.bin'"
expect "rename numbers file" 0 bash -c "'$bin' --no-config --allow-private -q --if-exists rename '$base/hello.txt'; test -f 'hello (1).txt'"
expect "404 fails" 1 r -q "$base/missing"
expect "private blocked by default" 1 "$bin" --no-config -q "$base/hello.txt"
rm -f hello.txt
expect "bad URL fails the run" 1 bash -c "printf '# comment\nnot a url\n$base/hello.txt\n' | '$bin' --no-config --allow-private -q -i -"
expect "bad URL does not stop batch" 0 test -f hello.txt

echo "e2e: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
