#!/usr/bin/env python3
"""Measurement harness for EXP-01.

For each encoding (peano, binary) and each size N in {10, 100, 500, 1000}:
  1. checks whether alias definitions ALONE compile at the default
     recursion_limit (128);
  2. finds the minimal #![recursion_limit] at which a crate that also
     *uses* the numbers compiles (evaluates N::VALUE and proves
     N/2 + N/2 == N by type equality);
  3. runs `cargo clean && time cargo build` three times at that minimal
     limit and reports the median, per the common protocol.

Scratch crates are generated under measure/work/ (safe to delete).
Results are printed as a Markdown table.
"""

import os
import shutil
import statistics
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
WORK = os.path.join(HERE, "work")
SIZES = [10, 100, 500, 1000]
DEFAULT_LIMIT = 128

PEANO_CORE = """
use core::marker::PhantomData;
pub struct Zero;
pub struct Succ<N>(PhantomData<N>);
pub trait Nat { const VALUE: u64; }
impl Nat for Zero { const VALUE: u64 = 0; }
impl<N: Nat> Nat for Succ<N> { const VALUE: u64 = N::VALUE + 1; }
pub trait Add<B: Nat>: Nat { type Sum: Nat; }
impl<B: Nat> Add<B> for Zero { type Sum = B; }
impl<A: Add<B>, B: Nat> Add<B> for Succ<A> { type Sum = Succ<A::Sum>; }
pub trait SameAs<T> {}
impl<T> SameAs<T> for T {}
pub fn same<A: SameAs<B>, B>() {}
"""

BINARY_CORE = """
use core::marker::PhantomData;
pub struct B0;
pub struct B1;
pub trait Bit { const BIT: u64; }
impl Bit for B0 { const BIT: u64 = 0; }
impl Bit for B1 { const BIT: u64 = 1; }
pub struct UTerm;
pub struct UInt<U, B>(PhantomData<(U, B)>);
pub trait Nat { const VALUE: u64; }
impl Nat for UTerm { const VALUE: u64 = 0; }
impl<U: Nat, B: Bit> Nat for UInt<U, B> { const VALUE: u64 = 2 * U::VALUE + B::BIT; }
pub trait Inc: Nat { type Out: Nat; }
impl Inc for UTerm { type Out = UInt<UTerm, B1>; }
impl<U: Nat> Inc for UInt<U, B0> { type Out = UInt<U, B1>; }
impl<U: Inc> Inc for UInt<U, B1> { type Out = UInt<U::Out, B0>; }
pub trait Add<Rhs: Nat>: Nat { type Sum: Nat; }
pub trait AddCarry<Rhs: Nat>: Nat { type Sum: Nat; }
impl Add<UTerm> for UTerm { type Sum = UTerm; }
impl<U: Nat, B: Bit> Add<UInt<U, B>> for UTerm { type Sum = UInt<U, B>; }
impl<U: Nat, B: Bit> Add<UTerm> for UInt<U, B> { type Sum = UInt<U, B>; }
impl<Ul: Add<Ur>, Ur: Nat> Add<UInt<Ur, B0>> for UInt<Ul, B0> { type Sum = UInt<Ul::Sum, B0>; }
impl<Ul: Add<Ur>, Ur: Nat> Add<UInt<Ur, B1>> for UInt<Ul, B0> { type Sum = UInt<Ul::Sum, B1>; }
impl<Ul: Add<Ur>, Ur: Nat> Add<UInt<Ur, B0>> for UInt<Ul, B1> { type Sum = UInt<Ul::Sum, B1>; }
impl<Ul: AddCarry<Ur>, Ur: Nat> Add<UInt<Ur, B1>> for UInt<Ul, B1> { type Sum = UInt<Ul::Sum, B0>; }
impl AddCarry<UTerm> for UTerm { type Sum = UInt<UTerm, B1>; }
impl<U: Inc> AddCarry<UInt<U, B0>> for UTerm { type Sum = UInt<U, B1>; }
impl<U: Inc> AddCarry<UInt<U, B1>> for UTerm { type Sum = UInt<U::Out, B0>; }
impl<U: Inc> AddCarry<UTerm> for UInt<U, B0> { type Sum = UInt<U, B1>; }
impl<U: Inc> AddCarry<UTerm> for UInt<U, B1> { type Sum = UInt<U::Out, B0>; }
impl<Ul: Add<Ur>, Ur: Nat> AddCarry<UInt<Ur, B0>> for UInt<Ul, B0> { type Sum = UInt<Ul::Sum, B1>; }
impl<Ul: AddCarry<Ur>, Ur: Nat> AddCarry<UInt<Ur, B1>> for UInt<Ul, B0> { type Sum = UInt<Ul::Sum, B0>; }
impl<Ul: AddCarry<Ur>, Ur: Nat> AddCarry<UInt<Ur, B0>> for UInt<Ul, B1> { type Sum = UInt<Ul::Sum, B0>; }
impl<Ul: AddCarry<Ur>, Ur: Nat> AddCarry<UInt<Ur, B1>> for UInt<Ul, B1> { type Sum = UInt<Ul::Sum, B1>; }
pub trait SameAs<T> {}
impl<T> SameAs<T> for T {}
pub fn same<A: SameAs<B>, B>() {}
"""


def peano_aliases(n):
    lines = ["pub type N0 = Zero;"]
    for i in range(1, n + 1):
        lines.append("pub type N%d = Succ<N%d>;" % (i, i - 1))
    return "\n".join(lines)


def binary_aliases(n):
    def spell(v):
        t = "UTerm"
        for bit in format(v, "b"):
            t = "UInt<%s, B%s>" % (t, bit)
        return t

    lines = ["pub type N0 = UTerm;"]
    for i in range(1, n + 1):
        lines.append("pub type N%d = %s;" % (i, spell(i)))
    return "\n".join(lines)


def crate_source(encoding, n, use, limit):
    core = PEANO_CORE if encoding == "peano" else BINARY_CORE
    aliases = peano_aliases(n) if encoding == "peano" else binary_aliases(n)
    parts = []
    if limit is not None:
        parts.append('#![recursion_limit = "%d"]' % limit)
    parts.append(core)
    parts.append(aliases)
    if use:
        parts.append("pub const TOP: u64 = N%d::VALUE;" % n)
        parts.append(
            "pub fn check() { same::<<N%d as Add<N%d>>::Sum, N%d>(); }"
            % (n // 2, n // 2, n)
        )
    return "\n".join(parts) + "\n"


def write_crate(path, source):
    os.makedirs(os.path.join(path, "src"), exist_ok=True)
    with open(os.path.join(path, "Cargo.toml"), "w") as f:
        f.write(
            '[package]\nname = "measure"\nversion = "0.1.0"\nedition = "2024"\n'
            "[dependencies]\n"
        )
    with open(os.path.join(path, "src", "lib.rs"), "w") as f:
        f.write(source)


def build(path):
    """Returns (ok, seconds) for cargo clean && cargo build."""
    subprocess.run(
        ["cargo", "clean"], cwd=path, capture_output=True, check=True
    )
    t0 = time.monotonic()
    r = subprocess.run(["cargo", "build"], cwd=path, capture_output=True)
    return r.returncode == 0, time.monotonic() - t0


def compiles(path, encoding, n, use, limit):
    write_crate(path, crate_source(encoding, n, use, limit))
    ok, _ = build(path)
    return ok


def minimal_limit(path, encoding, n):
    """Binary search for the smallest recursion_limit that compiles the
    'use' crate. Returns (limit, hit_default) where hit_default is True if
    the default (no attribute) already works."""
    if compiles(path, encoding, n, True, None):
        # Default works; still search below 128 for the true minimum.
        lo, hi = 1, DEFAULT_LIMIT
        default_ok = True
    else:
        default_ok = False
        hi = DEFAULT_LIMIT
        while not compiles(path, encoding, n, True, hi):
            hi *= 2
            if hi > 65536:
                return None, False
        lo = hi // 2
    while lo < hi:
        mid = (lo + hi) // 2
        if compiles(path, encoding, n, True, mid):
            hi = mid
        else:
            lo = mid + 1
    return hi, default_ok


def timed_builds(path, encoding, n, limit, runs=3):
    write_crate(path, crate_source(encoding, n, True, limit))
    times = []
    for _ in range(runs):
        ok, secs = build(path)
        if not ok:
            raise RuntimeError("build failed during timing")
        times.append(secs)
    return times


def main():
    rustc = subprocess.run(
        ["rustc", "--version"], capture_output=True, text=True
    ).stdout.strip()
    print("toolchain: %s\n" % rustc)
    os.makedirs(WORK, exist_ok=True)
    rows = []
    for encoding in ["peano", "binary"]:
        for n in SIZES:
            path = os.path.join(WORK, "crate")
            # 1. defs-only at default limit
            defs_ok = compiles(path, encoding, n, False, None)
            # 2. minimal limit for full use
            lim, default_ok = minimal_limit(path, encoding, n)
            # 3. timing at that limit
            times = timed_builds(path, encoding, n, lim)
            med = statistics.median(times)
            rows.append(
                (encoding, n, defs_ok, lim, default_ok, med, times)
            )
            print(
                "%s N=%d: defs-only@128 %s | min limit %s (default ok: %s) "
                "| build median %.2fs %s"
                % (
                    encoding,
                    n,
                    "ok" if defs_ok else "FAIL",
                    lim,
                    default_ok,
                    med,
                    ["%.2f" % t for t in times],
                )
            )
            sys.stdout.flush()

    print("\n| Encoding | N | Aliases-only compiles at default limit (128) | "
          "Minimal recursion_limit for use | Median clean build (3 runs) |")
    print("|---|---|---|---|---|")
    for encoding, n, defs_ok, lim, default_ok, med, times in rows:
        lim_s = "default is enough (min %d)" % lim if default_ok else str(lim)
        print(
            "| %s | %d | %s | %s | %.2f s |"
            % (encoding, n, "yes" if defs_ok else "NO", lim_s, med)
        )
    shutil.rmtree(WORK, ignore_errors=True)


if __name__ == "__main__":
    main()
