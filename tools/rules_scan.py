#!/usr/bin/env python3
"""rules_scan: scan the ADDED lines of a diff for banned constructs in ALL Rust code of zenith-bound.

Derived from zenith-float tools/rules_scan.py (commit d12a156), adapted for zenith-bound. Run on EVERY
model-written diff before review, and in tools/full_gate.sh (step `rules-scan`).

Usage:
  tools/rules_scan.sh                      # origin/main..HEAD, plus the two count-baseline checks
  tools/rules_scan.sh A..B                 # any range (a bare rev means REV..HEAD)
  tools/rules_scan.sh --staged             # git diff --cached
  tools/rules_scan.sh --working            # git diff HEAD (staged + unstaged; new files must be staged)
  tools/rules_scan.sh --diff-file F        # a saved unified diff (used by the self-test)
  tools/rules_scan.sh --count-only         # only the clone-count and *_ref-count baseline checks
  tools/rules_scan.sh --update-baseline    # lower the baselines to the current counts (never raises them)

SCANNED PATHS: every .rs file under src/ tests/ benches/ examples/ (any depth) and build.rs. There is NO
test-code exemption: #[cfg(test)] items, tests/, benches/, examples/ and test helpers are scanned exactly
like library code, and every pattern below applies to every scanned path.

Flagged on added lines only (comments and string literals are ignored):
  .clone(  .clone_from(  Clone::clone  .to_vec(  .cloned(  .to_owned(  derive(..Clone|Copy..)
  impl Clone|Copy for  unsafe  #[allow / #![allow  unwrap(  expect(  panic!  unreachable!  todo!
  unimplemented!  f32  f64  core::arch / std::arch / target_feature / mul_add / is_x86_feature_detected
(`assert!`, `assert_eq!`, `assert_ne!`, `assert_matches!` and their `debug_` forms ARE flagged, as are `unwrap_or_default(`, `unwrap_unchecked(`, `unwrap_err(`, `unwrap_err_unchecked(` and `expect_err(`. A `#[derive(` that spans several lines is flagged when a following added line of the same block names Clone or Copy.)

ALLOWLIST: tools/rules_allowlist.txt, one `path:hash<TAB>reason` per line; the key is printed as allow-key=
for every flagged site (hash = sha1 of the added line with whitespace collapsed). The file starts EMPTY.
ONLY JEFF MAY EXTEND IT (.github/CODEOWNERS requires his review). Models never add entries. An unwrap that
is absolutely necessary is listed there with its reason, for Jeff.

CLONE COUNT: total `.clone(` occurrences in all scanned Rust files (tests included) must never go up versus
tools/clone_count_baseline.txt (starts at 0). The baseline is only ever lowered, with --update-baseline.

REF COUNT: total uses of `add_ref(` `sub_ref(` `mul_ref(` `div_ref(` (borrowed-borrowed zenith-float calls,
sanctioned only where both operands are still live; mul_ref/div_ref only when absolutely necessary, each
use listed with its reason for Jeff), definitions (`fn add_ref` etc.) excluded. Same ratchet against
tools/ref_sum_count_baseline.txt (starts at 0): it may only go down; --update-baseline lowers it.
"""
import hashlib, os, re, subprocess, sys, collections

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.environ.get("RULES_SCAN_ROOT") or os.path.dirname(HERE)

DIRS = ["src", "tests", "benches", "examples"]

LIB = re.compile(r"^(src|tests|benches|examples)/.*\.rs$")
BUILD_RS = re.compile(r"^build\.rs$")

PATTERNS = [
    ("clone", re.compile(r"\.clone\(")),
    ("clone_from", re.compile(r"\.clone_from\(")),
    ("clone_path", re.compile(r"\bClone::clone\b")),
    ("to_vec", re.compile(r"\.to_vec\(")),
    ("cloned", re.compile(r"\.cloned\(")),
    ("to_owned", re.compile(r"\.to_owned\(")),
    ("derive_clone", re.compile(r"#\[derive\([^)]*\b(?:Clone|Copy)\b")),
    ("impl_clone", re.compile(r"\bimpl(?:<[^>]*>)?\s+(?:Clone|Copy)\s+for\b")),
    ("unsafe", re.compile(r"\bunsafe\b")),
    ("allow", re.compile(r"#!?\[allow\b|^\s*#!?\[.*\ballow\(")),
    ("unwrap", re.compile(r"\bunwrap\(")),
    ("expect", re.compile(r"\bexpect\(")),
    ("panic", re.compile(r"\bpanic!")),
    ("unreachable", re.compile(r"\bunreachable!")),
    ("todo", re.compile(r"\btodo!")),
    ("unimplemented", re.compile(r"\bunimplemented!")),
    ("unwrap_variant", re.compile(r"\bunwrap_or_default\(|\bunwrap_unchecked\(|\bunwrap_err\(|\bunwrap_err_unchecked\(|\bexpect_err\(")),
    ("assert", re.compile(r"\b(?:debug_)?assert(?:_eq|_ne)?!|\bassert_matches!")),
    ("f32", re.compile(r"\bf32\b|\d_?f32\b")),
    ("f64", re.compile(r"\bf64\b|\d_?f64\b")),
    ("hwfloat", re.compile(r"\b(?:core|std)::arch\b|\btarget_feature\b|\bmul_add\b|\bis_x86_feature_detected\b")),
]
STR = re.compile(r'"(?:\\.|[^"\\])*"')
REF_SUM = re.compile(r"\b(?:add_ref|sub_ref|mul_ref|div_ref)\(")
DEF_EXCLUSION = re.compile(r"\bfn\s+(add_ref|sub_ref|mul_ref|div_ref)\b")
DERIVE_OPEN = re.compile(r"#\[derive\((?![^)]*\)\])")
DERIVE_NAME = re.compile(r"\b(?:Clone|Copy)\b")


def is_rust(path):
    return bool(LIB.match(path)) or bool(BUILD_RS.match(path))


def strip(line):
    s = STR.sub('""', line)
    s = re.sub(r"/\*.*?\*/", "", s)
    return s.split("//")[0]


def git(*args):
    return subprocess.run(["git", "-C", ROOT, *args], capture_output=True, text=True)


def parse_diff(text):
    """yield (path, newline_no, content) for each added line"""
    path, n = None, 0
    for raw in text.split("\n"):
        if raw.startswith("+++ "):
            path = raw[4:].strip()
            path = path[2:] if path.startswith("b/") else (None if path == "/dev/null" else path)
        elif raw.startswith("--- ") or raw.startswith("diff --git"):
            continue
        elif raw.startswith("@@"):
            m = re.search(r"\+(\d+)", raw)
            n = int(m.group(1)) if m else 0
        elif path and raw.startswith("+"):
            yield path, n, raw[1:]
            n += 1
        elif path and raw.startswith(" "):
            n += 1


def load_allow():
    out = set()
    p = os.path.join(ROOT, "tools", "rules_allowlist.txt")
    if os.path.exists(p):
        for l in open(p):
            l = l.rstrip("\n")
            if l.strip() and not l.lstrip().startswith("#"):
                out.add(l.split("\t")[0].strip())
    return out


def scan_diff(text):
    allow, hits = load_allow(), []
    derive_path, derive_next = None, 0
    for path, n, content in parse_diff(text):
        if not is_rust(path):
            continue
        code = strip(content)
        key = "%s:%s" % (path, hashlib.sha1(" ".join(content.split()).encode()).hexdigest()[:12])
        if derive_path == path and n == derive_next:
            if DERIVE_NAME.search(code) and key not in allow:
                hits.append((path, n, "derive_clone", " ".join(content.split()), key))
            if ")]" in code:
                derive_path = None
            else:
                derive_next = n + 1
        else:
            derive_path = None
        if DERIVE_OPEN.search(code):
            derive_path = path
            derive_next = n + 1
        for label, rx in PATTERNS:
            if rx.search(code) and key not in allow:
                hits.append((path, n, label, " ".join(content.split()), key))
    return hits


def clone_count():
    total = 0
    for d in DIRS:
        for dp, dn, fn in os.walk(os.path.join(ROOT, d)):
            dn[:] = [x for x in dn if x not in ("target", ".git")]
            for f in fn:
                if not f.endswith(".rs"):
                    continue
                rel = os.path.relpath(os.path.join(dp, f), ROOT)
                if not is_rust(rel):
                    continue
                lines = open(os.path.join(dp, f), errors="replace").read().split("\n")
                for l in lines:
                    total += len(re.findall(r"\.clone\(", strip(l)))
    build_rs_path = os.path.join(ROOT, "build.rs")
    if os.path.exists(build_rs_path):
        lines = open(build_rs_path, errors="replace").read().split("\n")
        for l in lines:
            total += len(re.findall(r"\.clone\(", strip(l)))
    return total


def ref_sum_count():
    total = 0
    for d in DIRS:
        for dp, dn, fn in os.walk(os.path.join(ROOT, d)):
            dn[:] = [x for x in dn if x not in ("target", ".git")]
            for f in fn:
                if not f.endswith(".rs"):
                    continue
                rel = os.path.relpath(os.path.join(dp, f), ROOT)
                if not is_rust(rel):
                    continue
                lines = open(os.path.join(dp, f), errors="replace").read().split("\n")
                for l in lines:
                    code = strip(l)
                    if not DEF_EXCLUSION.search(code):
                        total += len(REF_SUM.findall(code))
    build_rs_path = os.path.join(ROOT, "build.rs")
    if os.path.exists(build_rs_path):
        lines = open(build_rs_path, errors="replace").read().split("\n")
        for l in lines:
            code = strip(l)
            if not DEF_EXCLUSION.search(code):
                total += len(REF_SUM.findall(code))
    return total


def baseline_path():
    return os.path.join(ROOT, "tools", "clone_count_baseline.txt")


def ref_baseline_path():
    return os.path.join(ROOT, "tools", "ref_sum_count_baseline.txt")


def read_baseline():
    try:
        return int(open(baseline_path()).read().split()[0])
    except (OSError, ValueError, IndexError):
        return None


def count_check(update=False):
    cur, base = clone_count(), read_baseline()
    if base is None:
        print("rules_scan: missing or unreadable tools/clone_count_baseline.txt")
        return 1
    if cur > base:
        print("rules_scan: clone count %d is ABOVE the baseline %d (it may only go down)" % (cur, base))
        return 1
    if cur < base:
        if update:
            open(baseline_path(), "w").write("%d\n" % cur)
            print("rules_scan: baseline lowered %d -> %d" % (base, cur))
        else:
            print("rules_scan: clone count %d is below baseline %d: lower the baseline (rules_scan.sh --update-baseline)" % (cur, base))
        return 0
    print("rules_scan: clone count %d == baseline" % cur)
    return 0


def ref_count_check(update=False):
    cur = ref_sum_count()
    try:
        base = int(open(ref_baseline_path()).read().split()[0])
    except (OSError, ValueError, IndexError):
        print("rules_scan: missing or unreadable tools/ref_sum_count_baseline.txt")
        return 1
    if cur > base:
        print("rules_scan: add_ref/sub_ref/mul_ref/div_ref use count %d is ABOVE the baseline %d (it may only go down)" % (cur, base))
        return 1
    if cur < base:
        if update:
            open(ref_baseline_path(), "w").write("%d\n" % cur)
            print("rules_scan: ref-sum baseline lowered %d -> %d" % (base, cur))
        else:
            print("rules_scan: add_ref/sub_ref/mul_ref/div_ref use count %d is below baseline %d: lower the baseline (rules_scan.sh --update-baseline)" % (cur, base))
        return 0
    print("rules_scan: add_ref/sub_ref/mul_ref/div_ref use count %d == baseline" % cur)
    return 0


def both_count_checks(update=False):
    return max(count_check(update), ref_count_check(update))


def main(argv):
    if "--count-only" in argv:
        return both_count_checks()
    if "--update-baseline" in argv:
        return both_count_checks(update=True)
    with_count = False
    if "--diff-file" in argv:
        text = open(argv[argv.index("--diff-file") + 1], errors="replace").read()
    elif "--staged" in argv:
        text = git("diff", "--cached", "--no-color", "--no-ext-diff", "-U0").stdout
    elif "--working" in argv:
        text = git("diff", "HEAD", "--no-color", "--no-ext-diff", "-U0").stdout
    else:
        args = [a for a in argv if not a.startswith("--")]
        rng = args[0] if args else "origin/main..HEAD"
        if ".." not in rng:
            rng += "..HEAD"
        r = git("diff", rng, "--no-color", "--no-ext-diff", "-U0")
        if r.returncode != 0:
            print("rules_scan: git diff %s failed: %s" % (rng, r.stderr.strip()))
            return 2
        text, with_count = r.stdout, True
    hits = scan_diff(text)
    for path, n, label, content, key in hits:
        print("%s:%d [%s] %s  allow-key=%s" % (path, n, label, content[:120], key))
    by = collections.Counter(h[2] for h in hits)
    print("rules_scan: %d flagged %s" % (len(hits), dict(by)))
    rc = 1 if hits else 0
    if with_count and not os.environ.get("RULES_SCAN_SKIP_COUNT"):
        rc = max(rc, both_count_checks())
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
