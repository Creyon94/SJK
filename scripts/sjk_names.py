"""SJK's names, and merges from JKR that keep them.

SJK's crates are JKR's under other names: `crates/jkr-bsp` is `crates/sjk-bsp`, the
package `jkr-bsp` is `sjk-bsp` and the Rust crate `jkr_bsp` is `sjk_bsp`. JKR's `jkr_*`
settings and commands have engine names in SJK (`jkr_volumetrics` is `r_volumetrics`),
listed in SJK's alias tables (RENAME_TABLES below). The renames are mechanical, so the
same rules translate any JKR commit to SJK's names. A merge from JKR translates both
JKR's side and the common ancestor first, then merges three ways under SJK's names: a
JKR change merges as cleanly as it would have without the renames.

    python scripts/sjk_names.py apply [DIR]        apply the names to the checkout in DIR
    python scripts/sjk_names.py merge REF [-m MSG]  merge a JKR commit into HEAD
    python scripts/sjk_names.py continue            commit a merge after resolving conflicts
    python scripts/sjk_names.py abort               drop a merge left for resolution

`merge` runs in a clean checkout of SJK's `main` (or a branch of it). REF is a JKR
commit, such as `upstream/main` or a JKR pull request branch. The merge commit keeps
REF itself as its second parent, so git history still shows what was merged. If the
three-way merge has conflicts, the checkout is left with them for resolution.

These keep JKR's spelling on purpose: the old names inside the alias tables, the
`GameData/jkr` import, `JKR_*` environment variables, the dedicated server's
`jkr_server.cfg`, and "JKR" naming Bishop's project.
"""

import argparse, os, re, shutil, subprocess, sys, tempfile

CRATES = ["audio", "bsp", "client", "dedicated", "effect", "entity", "game-jka", "icarus",
          "materialgen", "model", "nav", "network", "protocol", "runtime", "scene", "server",
          "shader", "shell", "ui", "vfs", "viewer"]
_NAMES = "|".join(c.replace("-", "[-_]") for c in sorted(CRATES, key=len, reverse=True))
# jkr-bsp, jkr_bsp, crates/jkr-bsp; not jkr_server.cfg (the server's saved settings) and
# not inside a longer identifier (foo_jkr_bsp).
CRATE_NAME = re.compile(rb"(?<![\w-])jkr([-_](?:" + _NAMES.encode() + rb"))(?!\.cfg)\b")
RULES_VERSION = 5  # bump when the rules change; every translation must use the same rules
# Text that looks like a crate name but names one of JKR's own programs: kept as written.
KEEP = [b'JKR_SERVER_BINARY: &str = "jkr-dedicated"',
        b"jkr-materialgen/manifest.json"]  # the path inside existing material packs

# Files whose ("jkr_old", "new") pairs are SJK's renamed settings and commands. They keep
# the old names (as aliases), so the setting rule skips them; the crate rule does not.
RENAME_TABLES = ["crates/{viewer}/src/cvar_renames.rs", "crates/{dedicated}/src/cvars/mod.rs"]
# This file describes the rules with examples, which must stay as written.
SKIP = {"scripts/sjk_names.py"}
# A table entry is a tuple of its own, `("jkr_old", "new"),`; a call such as
# `cvars.set(b"jkr_stockRules", b"1")` in the tables' tests is not one.
TABLE_PAIR = re.compile(r'(?<![\w.])\(b?"(jkr_\w+)",\s*b?"(\w+)"\)')

STATE = "SJK_NAMES_MERGE"


def run(*cmd, cwd=None, check=True, inp=None):
    r = subprocess.run(cmd, cwd=cwd, capture_output=True, input=inp)
    if check and r.returncode != 0:
        sys.exit(f"{' '.join(cmd)} failed:\n{r.stderr.decode(errors='replace')}")
    return r


def git(*args, cwd=None, check=True, inp=None):
    return run("git", *args, cwd=cwd, check=check, inp=inp).stdout.decode().strip()


# --- the rules ------------------------------------------------------------------------------

def table_paths(root):
    names = {}
    for kind in ("viewer", "dedicated"):
        renamed = os.path.isdir(os.path.join(root, "crates", f"sjk-{kind}"))
        names[kind] = f"sjk-{kind}" if renamed else f"jkr-{kind}"
    return [t.format(**names) for t in RENAME_TABLES]


def setting_renames(root):
    """{old: new} from SJK's alias tables in the checkout at root."""
    pairs = {}
    for rel in table_paths(root):
        text = open(os.path.join(root, rel), encoding="utf-8").read()
        for old, new in TABLE_PAIR.findall(text):
            if pairs.setdefault(old, new) != new:
                sys.exit(f"{old} has two names in the alias tables: {pairs[old]} and {new}")
    if not pairs:
        sys.exit("No setting renames found: are the alias tables where RENAME_TABLES says?")
    return pairs


def setting_rule(pairs):
    names = "|".join(sorted((re.escape(o) for o in pairs), key=len, reverse=True))
    pattern = re.compile(rb"(?<!\w)(" + names.encode() + rb")(?!\w)")
    table = {o.encode(): n.encode() for o, n in pairs.items()}
    return lambda data: pattern.sub(lambda m: table[m[1]], data)


def rename_text(data, settings=None):
    kept = {}
    for i, text in enumerate(KEEP):
        if text in data:
            kept[b"\0KEEP%d\0" % i] = text
            data = data.replace(text, b"\0KEEP%d\0" % i)
    data = CRATE_NAME.sub(rb"sjk\1", data)
    data = settings(data) if settings else data
    for marker, text in kept.items():
        data = data.replace(marker, text)
    return data


# --- Cargo.lock: cargo sorts packages by name, so renamed ones must move ----------------

def _version_key(version):
    core, _, pre = version.partition("-")
    nums = [int(p) if p.isdigit() else 0 for p in core.split(".")]
    return nums, pre == "", pre


def sort_lockfile(data):
    text = data.decode()
    newline = "\r\n" if "\r\n" in text else "\n"
    text = text.replace("\r\n", "\n")
    head, *blocks = text.split("\n[[package]]\n")
    tail = ""
    if blocks and "\n[metadata]" in blocks[-1]:
        blocks[-1], meta = blocks[-1].split("\n[metadata]", 1)
        tail = "\n[metadata]" + meta

    def field(block, name):
        m = re.search(rf'^{name} = "([^"]*)"', block, re.M)
        return m[1] if m else ""

    def dep_key(entry):
        parts = entry.strip().strip(",").strip('"').split(" ")
        return parts[0], _version_key(parts[1]) if len(parts) > 1 else ([], True, "")

    def sort_deps(block):
        def fix(m):
            entries = [e for e in m[1].split("\n") if e.strip()]
            return "dependencies = [\n" + "\n".join(sorted(entries, key=dep_key)) + "\n]"
        return re.sub(r"dependencies = \[\n(.*?)\n\]", fix, block, flags=re.S)

    blocks = [sort_deps(b.rstrip("\n")) for b in blocks]
    blocks.sort(key=lambda b: (field(b, "name"), _version_key(field(b, "version")),
                               field(b, "source")))
    out = head + "".join("\n[[package]]\n" + b + "\n" for b in blocks).rstrip("\n") + "\n" + tail
    return out.replace("\n", newline).encode()


# --- applying the rules to a checkout ---------------------------------------------------

def apply(root, pairs=None):
    """Apply SJK's names to the checkout at root, in place (index and files). The setting
    renames come from `pairs`, or else from root's own alias tables."""
    settings = setting_rule(pairs or setting_renames(root))
    for crate in CRATES:
        if os.path.isdir(os.path.join(root, "crates", f"jkr-{crate}")):
            git("mv", f"crates/jkr-{crate}", f"crates/sjk-{crate}", cwd=root)
    tables = set(table_paths(root))
    changed_rust = []
    for rel in filter(None, git("ls-files", "-z", cwd=root).split("\0")):
        path = os.path.join(root, rel)
        if rel in SKIP or not os.path.isfile(path) or os.path.islink(path):
            continue
        data = open(path, "rb").read()
        if b"\0" in data[:8000]:
            continue
        new = rename_text(data, None if rel in tables else settings)
        if rel == "Cargo.lock":
            new = sort_lockfile(new)
        if new != data:
            open(path, "wb").write(new)
            if rel.endswith(".rs"):
                changed_rust.append(rel)
    # `use` lines are sorted by name, and sjk_* sorts elsewhere than jkr_* did.
    for i in range(0, len(changed_rust), 100):
        run("rustfmt", "--edition", "2024", *changed_rust[i:i + 100], cwd=root)
    git("add", "-A", cwd=root)


def translate(commit, repo, pairs):
    """A commit whose tree is `commit`'s under SJK's names (parent: commit)."""
    tmp = tempfile.mkdtemp(prefix="sjk-names-")
    try:
        git("worktree", "add", "-q", "--detach", tmp, commit, cwd=repo)
        apply(tmp, pairs)
        leftover = [d for d in os.listdir(os.path.join(tmp, "crates")) if d.startswith("jkr-")]
        if leftover:
            print(f"warning: {commit[:9]} has crates the rules do not know: "
                  f"{', '.join(leftover)}. Add them to CRATES (and bump RULES_VERSION).")
        tree = git("write-tree", cwd=tmp)
    finally:
        git("worktree", "remove", "--force", tmp, cwd=repo, check=False)
        shutil.rmtree(tmp, ignore_errors=True)
    return git("commit-tree", tree, "-p", commit, "-m",
               f"SJK names (rules v{RULES_VERSION}) of {commit}", cwd=repo)


# --- merging ------------------------------------------------------------------------------

def state_path(repo):
    return os.path.join(git("rev-parse", "--absolute-git-dir", cwd=repo), STATE)


def merge(ref, message, repo):
    if git("status", "--porcelain", "--untracked-files=no", cwd=repo):
        sys.exit("The checkout has uncommitted changes; commit or stash them first.")
    if os.path.exists(state_path(repo)):
        sys.exit("A merge is waiting: resolve it and run `continue`, or run `abort`.")
    head, theirs = git("rev-parse", "HEAD", cwd=repo), git("rev-parse", ref, cwd=repo)
    base = git("merge-base", head, theirs, cwd=repo)
    if base == theirs:
        sys.exit(f"{ref} is already merged.")
    print(f"Translating {ref} and the merge base {base[:9]} to SJK names...")
    pairs = setting_renames(repo)
    t_base, t_theirs = translate(base, repo, pairs), translate(theirs, repo, pairs)
    r = run("git", "merge-tree", "--write-tree", "--name-only", "--merge-base", t_base,
            head, t_theirs, cwd=repo, check=False)
    out = r.stdout.decode().splitlines()
    if r.returncode not in (0, 1) or not out:
        sys.exit(r.stderr.decode(errors="replace"))
    tree = out[0]
    conflicted = []
    for line in out[1:]:
        if not line:
            break  # the informational messages follow the conflicted file list
        conflicted.append(line)
    branch = git("rev-parse", "--abbrev-ref", "HEAD", cwd=repo)
    message = message or f"Merge {ref} into {branch}"
    if r.returncode == 0:
        commit(tree, head, theirs, message, repo)
        return
    open(state_path(repo), "w", encoding="utf-8").write(f"{head}\n{theirs}\n{message}\n")
    git("read-tree", "-u", "--reset", tree, cwd=repo)
    print("Conflicts (files contain conflict markers):")
    for path in sorted(set(conflicted)):
        print(f"  {path}")
    print("Resolve them, `git add` the files, then run `continue` (or `abort`).")
    sys.exit(1)


def commit(tree, head, theirs, message, repo):
    new = git("commit-tree", tree, "-p", head, "-p", theirs, "-F", "-", cwd=repo,
              inp=message.encode())
    git("update-ref", "-m", f"sjk_names merge: {message}", "HEAD", new, head, cwd=repo)
    git("reset", "-q", "--hard", new, cwd=repo)
    print(f"Merged: {git('log', '--oneline', '-1', new, cwd=repo)}")
    print("If JKR changed dependencies, check Cargo.lock with `cargo build --locked`.")


def cont(repo):
    path = state_path(repo)
    if not os.path.exists(path):
        sys.exit("No merge is waiting.")
    head, theirs, message = open(path, encoding="utf-8").read().split("\n", 2)
    if git("rev-parse", "HEAD", cwd=repo) != head:
        sys.exit("HEAD moved since the merge started; run `abort` and merge again.")
    marked = run("git", "grep", "-l", "-E", "^(<<<<<<<|>>>>>>>) ", "--cached", cwd=repo,
                 check=False).stdout.decode().split()
    if marked:
        sys.exit("Conflict markers remain in: " + ", ".join(marked))
    tree = git("write-tree", cwd=repo)
    os.remove(path)
    commit(tree, head, theirs, message.strip(), repo)


def abort(repo):
    path = state_path(repo)
    if not os.path.exists(path):
        sys.exit("No merge is waiting.")
    head = open(path, encoding="utf-8").read().split("\n")[0]
    git("reset", "-q", "--hard", head, cwd=repo)
    os.remove(path)
    print("Merge dropped.")


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    a = sub.add_parser("apply")
    a.add_argument("dir", nargs="?", default=".")
    m = sub.add_parser("merge")
    m.add_argument("ref")
    m.add_argument("-m", "--message")
    sub.add_parser("continue")
    sub.add_parser("abort")
    args = ap.parse_args()
    if args.cmd == "apply":
        apply(git("rev-parse", "--show-toplevel", cwd=os.path.abspath(args.dir)))
        return
    repo = git("rev-parse", "--show-toplevel")
    {"merge": lambda: merge(args.ref, args.message, repo),
     "continue": lambda: cont(repo), "abort": lambda: abort(repo)}[args.cmd]()


if __name__ == "__main__":
    main()
