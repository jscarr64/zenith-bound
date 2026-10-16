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
  tools/rules_scan.sh --count-only         # only the clone-count, *_ref-count and hidden-copy-count baseline checks
  tools/rules_scan.sh --hidden-copies      # list the hidden-copy sites of src/ (no baseline needed; RULES_SCAN_ROOT picks the tree)
  tools/rules_scan.sh --update-baseline    # lower the baselines to the current counts (never raises them)

SCANNED PATHS: every .rs file under src/ tests/ benches/ examples/ (any depth) and build.rs. There is NO
test-code exemption: #[cfg(test)] items, tests/, benches/, examples/ and test helpers are scanned exactly
like library code, and every pattern below applies to every scanned path.

Flagged on added lines only (comments and string literals are ignored):
  .clone(  .clone_from(  Clone::clone  .to_vec(  .cloned(  .copied(  .to_owned(  derive(..Clone|Copy..)
  impl Clone|Copy for  unsafe  #[allow / #![allow  unwrap(  expect(  panic!  unreachable!  todo!
  unimplemented!  f32  f64  core::arch / std::arch / target_feature / mul_add / is_x86_feature_detected
(`assert!`, `assert_eq!`, `assert_ne!`, `assert_matches!` and their `debug_` forms ARE flagged, as are `unwrap_or_default(`, `unwrap_unchecked(`, `unwrap_err(`, `unwrap_err_unchecked(` and `expect_err(`. A `#[derive(` that spans several lines is flagged when a following added line of the same block names Clone or Copy.)

ALLOWLIST: tools/rules_allowlist.txt, one `path:hash<TAB>reason` per line; the key is printed as allow-key=
for every flagged site (hash = sha1 of the added line with whitespace collapsed). The file starts EMPTY.
ONLY JEFF MAY EXTEND IT (.github/CODEOWNERS requires his review). Models never add entries. An unwrap that
is absolutely necessary is listed there with its reason, for Jeff.

CLONE COUNT: total `.clone(` and `.copied(` occurrences in all scanned Rust files (tests included) must never go up versus
tools/clone_count_baseline.txt (starts at 0). The baseline is only ever lowered, with --update-baseline.

REF COUNT: total uses of `add_ref(` `sub_ref(` `mul_ref(` `div_ref(` (borrowed-borrowed zenith-float calls,
sanctioned only where both operands are still live; mul_ref/div_ref only when absolutely necessary, each
use listed with its reason for Jeff), definitions (`fn add_ref` etc.) excluded. Same ratchet against
tools/ref_sum_count_baseline.txt (starts at 0): it may only go down; --update-baseline lowers it.

HIDDEN-COPY COUNT: a `Mag` (or any type) has no Clone/Copy, but rebuilding it from the fields of another value of
the same type is a clone in effect. The count is the number of fn bodies under src/ (tests, benches and examples
are not counted) that
  (a) are NAMED copy, replicate, dup or duplicate (also copy_*, replicate_*, dup_*, duplicate_*), or
  (b) call a constructor (`Self {..}`, `Type {..}`, `Type::f(..)`, `Self::f(..)`; Some/Ok/Err excluded) whose arguments are
      two or more bare field reads of ONE value (`x.exp`, `x.mant()`, `*x.mant`, `a: x.a, b: x.b`), or
  (c) call such a constructor with two or more arguments that are bare names bound by a struct pattern of the same
      fn body (`Inner::Finite { exp, mant } => Mag::from_canonical(*exp, *mant)`).
Heuristic, text-based: arguments that are computed (`exp + 1`, `mant.checked_add(1)`) are not bare field reads, so
`successor`-style code does not count. A copy routed through intermediate `let` bindings is not seen. The count is
ratcheted against tools/hidden_copy_count_baseline.txt (starts at 0, may only go down; --update-baseline lowers it).
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
    ("copied", re.compile(r"\.copied\(")),
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
                    total += len(re.findall(r"\.(?:clone|copied)\(", strip(l)))
    build_rs_path = os.path.join(ROOT, "build.rs")
    if os.path.exists(build_rs_path):
        lines = open(build_rs_path, errors="replace").read().split("\n")
        for l in lines:
            total += len(re.findall(r"\.(?:clone|copied)\(", strip(l)))
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


FN_DECL = re.compile(r"\bfn\s+([A-Za-z_]\w*)")
COPY_NAME = re.compile(r"^(?:copy|replicate|dup|duplicate)(?:_\w+)?$")
CTOR = re.compile(r"\b((?:Self|[A-Z]\w*)(?:::\w+)*)\s*([({])")
NOT_CTOR = {"Some", "Ok", "Err", "Box", "Vec", "Option", "Result"}
BAD_PREV = {"struct", "enum", "impl", "for", "trait", "union", "mod", "type", "where", "dyn", "use"}
FIELD_READ = re.compile(r"^\*?\(?\*?(\w+)\.(\w+)(?:\(\))?\??\)?$")
BARE_NAME = re.compile(r"^[A-Za-z_]\w*$")


def blank_code(text):
    """text without comments, string and char literals (newlines kept, so line numbers stay)"""
    text = re.sub(r"/\*.*?\*/", lambda m: re.sub(r"[^\n]", " ", m.group(0)), text, flags=re.S)
    text = re.sub(r'"(?:\\.|[^"\\])*"', '""', text, flags=re.S)
    text = re.sub(r"'(?:\\.|[^'\\\n])'", "''", text)
    return re.sub(r"//[^\n]*", "", text)


def match_close(text, i):
    """index of the bracket closing the one at text[i], or -1"""
    pairs = {"(": ")", "{": "}", "[": "]"}
    stack = []
    for j in range(i, len(text)):
        c = text[j]
        if c in pairs:
            stack.append(pairs[c])
        elif c in ")}]":
            if not stack or stack.pop() != c:
                return -1
            if not stack:
                return j
    return -1


def split_top(s):
    out, depth, cur = [], 0, []
    for c in s:
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        if c == "," and depth == 0:
            out.append("".join(cur))
            cur = []
        else:
            cur.append(c)
    out.append("".join(cur))
    return [x.strip() for x in out if x.strip()]


def norm_arg(a):
    a = re.sub(r"^(?:\*|&mut\s+|&)+", "", a.strip()).strip()
    return a


def pattern_bound_names(body):
    """names bound by `Path { a, f: b, .. }` struct patterns (a brace group followed by =>, =, |, if or else)"""
    names = set()
    for m in CTOR.finditer(body):
        if m.group(2) != "{" or m.group(1).split("::")[0] in NOT_CTOR:
            continue
        close = match_close(body, m.end() - 1)
        if close < 0:
            continue
        after = body[close + 1:].lstrip()
        if not (after.startswith("=>") or re.match(r"=(?!=)", after) or after.startswith("|") or re.match(r"(?:if|else)\b", after)):
            continue
        for item in split_top(body[m.end():close]):
            if item == "..":
                continue
            item = item.split(":", 1)[1].strip() if ":" in item else item
            item = re.sub(r"^(?:ref\s+|mut\s+)+", "", item)
            if BARE_NAME.match(item) and item != "_":
                names.add(item)
    return names


def hidden_copy_reason(name, body):
    if COPY_NAME.match(name):
        return "fn named %s" % name
    bound = pattern_bound_names(body)
    for m in CTOR.finditer(body):
        path, opener = m.group(1), m.group(2)
        if path.split("::")[0] in NOT_CTOR:
            continue
        prev = re.findall(r"[A-Za-z_]\w*\s*$", body[:m.start()])
        if prev and prev[0].strip() in BAD_PREV:
            continue
        close = match_close(body, m.end() - 1)
        if close < 0:
            continue
        if opener == "{":
            after = body[close + 1:].lstrip()
            if after.startswith("=>") or re.match(r"=(?!=)", after) or after.startswith("|") or re.match(r"(?:if|else)\b", after):
                continue
        args = []
        for item in split_top(body[m.end():close]):
            if opener == "{":
                item = item.split(":", 1)[1].strip() if ":" in item else item
            args.append(norm_arg(item))
        bases = collections.defaultdict(set)
        for a in args:
            fr = FIELD_READ.match(a)
            if fr:
                bases[fr.group(1)].add(fr.group(2))
        for base, fields in bases.items():
            if len(fields) >= 2:
                return "%s(..) rebuilt from fields %s of `%s`" % (path, "/".join(sorted(fields)), base)
        got = {a for a in args if BARE_NAME.match(a) and a in bound}
        if len(got) >= 2:
            return "%s(..) rebuilt from pattern-bound %s" % (path, "/".join(sorted(got)))
    return None


def hidden_copy_sites():
    """[(relpath, line, fn name, reason)] for every flagged fn body under src/"""
    sites = []
    base = os.path.join(ROOT, "src")
    for dp, dn, fn in os.walk(base):
        dn[:] = sorted(x for x in dn if x not in ("target", ".git"))
        for f in sorted(fn):
            if not f.endswith(".rs"):
                continue
            path = os.path.join(dp, f)
            rel = os.path.relpath(path, ROOT)
            text = blank_code(open(path, errors="replace").read())
            for m in FN_DECL.finditer(text):
                i = m.end()
                depth = 0
                while i < len(text):
                    c = text[i]
                    if c in "([":
                        depth += 1
                    elif c in ")]":
                        depth -= 1
                    elif c == ";" and depth == 0:
                        i = -1
                        break
                    elif c == "{" and depth == 0:
                        break
                    i += 1
                if i < 0 or i >= len(text):
                    continue
                close = match_close(text, i)
                if close < 0:
                    continue
                reason = hidden_copy_reason(m.group(1), text[i + 1:close])
                if reason:
                    sites.append((rel, text.count("\n", 0, m.start()) + 1, m.group(1), reason))
    return sites


def hidden_copy_count():
    return len(hidden_copy_sites())


def hidden_baseline_path():
    return os.path.join(ROOT, "tools", "hidden_copy_count_baseline.txt")


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


def hidden_count_check(update=False):
    cur = hidden_copy_count()
    try:
        base = int(open(hidden_baseline_path()).read().split()[0])
    except (OSError, ValueError, IndexError):
        print("rules_scan: missing or unreadable tools/hidden_copy_count_baseline.txt")
        return 1
    if cur > base:
        print("rules_scan: hidden-copy count %d is ABOVE the baseline %d (it may only go down)" % (cur, base))
        for rel, line, name, reason in hidden_copy_sites():
            print("  %s:%d fn %s: %s" % (rel, line, name, reason))
        return 1
    if cur < base:
        if update:
            open(hidden_baseline_path(), "w").write("%d\n" % cur)
            print("rules_scan: hidden-copy baseline lowered %d -> %d" % (base, cur))
        else:
            print("rules_scan: hidden-copy count %d is below baseline %d: lower the baseline (rules_scan.sh --update-baseline)" % (cur, base))
        return 0
    print("rules_scan: hidden-copy count %d == baseline" % cur)
    return 0


def both_count_checks(update=False):
    return max(count_check(update), ref_count_check(update), hidden_count_check(update))


def main(argv):
    if "--hidden-copies" in argv:
        sites = hidden_copy_sites()
        for rel, line, name, reason in sites:
            print("%s:%d fn %s: %s" % (rel, line, name, reason))
        print("rules_scan: hidden_copy_count %d" % len(sites))
        return 1 if sites else 0
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
