"""Prepare and validate an upstream engine update without replacing the UI."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[1]
CANDIDATE = ROOT / "target/engine-sync/candidate.json"


def git(*args, data=None, check=True):
    return subprocess.run(
        ["git", *args], cwd=ROOT, input=data, stdout=subprocess.PIPE, check=check
    ).stdout


def matches(path, prefixes):
    return any(path == prefix or path.startswith(prefix.rstrip("/") + "/") for prefix in prefixes)


def protected_snapshot(policy):
    files = git("ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", *policy["protected_paths"])
    return {
        name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
        for name in files.decode().split("\0")
        if name and (ROOT / name).is_file()
    }


def manifest_identity(text, allowed):
    def strip(value, path=()):
        if not isinstance(value, dict):
            return value
        dependency_table = path and (path[-1] in {"dependencies", "build-dependencies"} or path[:2] == ("patch", "crates-io"))
        result = {}
        for key, item in value.items():
            if dependency_table and key in allowed:
                continue
            cleaned = strip(item, (*path, key))
            if cleaned != {}:
                result[key] = cleaned
        return result
    return strip(tomllib.loads(text))


def dependency_entries(text, allowed):
    lines = text.splitlines(keepends=True)
    entries, tables = {}, {}
    current, header = None, ""
    headers = []
    for index, line in enumerate(lines):
        stripped = line.strip()
        if re.fullmatch(r"\[.+\](?:\s*#.*)?", stripped):
            headers.append(index)
            if stripped.startswith("[["):
                current = None
                continue
            document = tomllib.loads(stripped + "\n__entry_marker__ = true\n")
            path = []
            while "__entry_marker__" not in document:
                if len(document) != 1:
                    raise ValueError("Unsupported Cargo table layout")
                key, document = next(iter(document.items()))
                path.append(key)
            current, header = tuple(path), stripped
            tables[current] = index
            continue
        found = re.match(r"\s*([A-Za-z0-9_-]+)\s*=", line)
        if not found or current is None or found[1] not in allowed:
            continue
        if not (current[-1] in {"dependencies", "build-dependencies"} or current[:2] == ("patch", "crates-io")):
            continue
        end = index + 1
        while True:
            raw = "".join(lines[index:end])
            try:
                parsed = tomllib.loads(header + "\n" + raw)
                break
            except tomllib.TOMLDecodeError:
                end += 1
                if end > len(lines):
                    raise ValueError(f"Unable to read Cargo dependency {found[1]}") from None
        value = parsed
        for key in (*current, found[1]):
            value = value[key]
        entries[(*current, found[1])] = (value, index, end, raw, header)
    table_ends = {
        path: next((index for index in headers if index > start), len(lines))
        for path, start in tables.items()
    }
    expected = set()
    def collect(document, path=()):
        if not isinstance(document, dict):
            return
        dependency_table = path and (path[-1] in {"dependencies", "build-dependencies"} or path[:2] == ("patch", "crates-io"))
        for key, value in document.items():
            if dependency_table and key in allowed:
                expected.add((*path, key))
            else:
                collect(value, (*path, key))
    collect(tomllib.loads(text))
    if set(entries) != expected:
        raise ValueError("Unsupported engine dependency table; reconcile Cargo.toml manually")
    return entries, table_ends


def update_dependencies(current, base, target, allowed):
    before_identity = manifest_identity(current, allowed)
    local, table_ends = dependency_entries(current, allowed)
    old, _ = dependency_entries(base, allowed)
    new, _ = dependency_entries(target, allowed)
    edits, additions = [], {}
    missing = object()
    changed = []
    for path in sorted(old.keys() | new.keys()):
        previous = old.get(path, (missing,))[0]
        incoming = new.get(path, (missing,))[0]
        existing = local.get(path, (missing,))[0]
        if previous == incoming or existing == incoming:
            continue
        if existing != previous:
            raise ValueError(f"Local engine dependency needs manual reconciliation: {'.'.join(path)}")
        changed.append(".".join(path))
        replacement = new[path][3] if path in new else ""
        if path in local:
            edits.append((local[path][1], local[path][2], replacement))
        else:
            additions.setdefault(path[:-1], []).append((new[path][4], replacement))
    lines = current.splitlines(keepends=True)
    for table, values in additions.items():
        position = table_ends.get(table, len(lines))
        prefix = "" if table in table_ends else "\n" + values[0][0] + "\n"
        edits.append((position, position, prefix + "".join(value for _, value in values)))
    for start, end, value in sorted(edits, reverse=True):
        lines[start:end] = [value]
    updated = "".join(lines)
    if manifest_identity(updated, allowed) != before_identity:
        raise ValueError("Engine update changed package identity or UI dependencies")
    return updated, changed


def verify_candidate(policy, candidate):
    if protected_snapshot(policy) != candidate["protected"]:
        raise ValueError("Protected UI, packaging, documentation or updater files changed")
    identity = manifest_identity((ROOT / "Cargo.toml").read_text(encoding="utf-8"), policy["engine_dependencies"])
    if identity != candidate["manifest_identity"]:
        raise ValueError("Package metadata or non-engine dependencies changed")


def finalize(policy):
    if not CANDIDATE.exists():
        raise ValueError("No pending engine candidate")
    candidate = json.loads(CANDIDATE.read_text(encoding="utf-8"))
    verify_candidate(policy, candidate)
    if git("diff", "--name-only", "--diff-filter=U").strip():
        raise ValueError("Resolve the engine conflicts before finalizing")
    subprocess.run(["cargo", "check", "--all-targets"], cwd=ROOT, check=True)
    subprocess.run([sys.executable, "packaging/verify-ui.py"], cwd=ROOT, check=True)
    verify_candidate(policy, candidate)
    source = json.loads((ROOT / "engine-source.json").read_text(encoding="utf-8"))
    source.update(integrated_ref=candidate["ref"], integrated_commit=candidate["commit"])
    (ROOT / "engine-source.json").write_text(json.dumps(source, indent=2) + "\n", encoding="utf-8")
    git("add", "--", "Cargo.toml", "Cargo.lock", "engine-source.json")
    CANDIDATE.unlink()
    print("Engine candidate validated; protected UI files and UI dependencies are unchanged. Review and commit the branch.")


def prepare(reference, apply, policy):
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._/-]{0,180}", reference) or ".." in reference:
        raise ValueError("Provide an upstream tag, branch or commit, not Git options")
    source = json.loads((ROOT / "engine-source.json").read_text(encoding="utf-8"))
    if git("remote", "get-url", "upstream").decode().strip() != source["upstream"]:
        raise ValueError("The upstream remote differs from engine-source.json")
    if apply and git("status", "--porcelain").strip():
        raise ValueError("Commit or stash your work before applying an engine update")
    git("fetch", "--no-tags", "upstream", reference)
    target = git("rev-parse", "FETCH_HEAD").decode().strip()
    base = source["integrated_commit"]
    current_manifest = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    base_manifest = git("show", f"{base}:Cargo.toml").decode()
    target_manifest = git("show", f"{target}:Cargo.toml").decode()
    protected = protected_snapshot(policy)
    candidate = {"ref": reference, "commit": target, "protected": protected,
                 "manifest_identity": manifest_identity(current_manifest, policy["engine_dependencies"])}
    all_paths = git("diff", "--name-only", "--no-renames", base, target).decode().splitlines()
    changes = [path for path in all_paths if matches(path, policy["engine_paths"])]
    output = ROOT / "target/engine-sync"
    output.mkdir(parents=True, exist_ok=True)
    plan = {"from": base, "to": target, "reference": reference,
            "engine_files": changes,
            "excluded_files": [path for path in all_paths if path not in changes and path not in {"Cargo.toml", "Cargo.lock"}]}
    (output / "plan.json").write_text(json.dumps(plan, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(plan, indent=2))
    updated_manifest, dependencies = update_dependencies(current_manifest, base_manifest, target_manifest, policy["engine_dependencies"])
    print("Engine dependencies:", ", ".join(dependencies) or "unchanged")
    (output / "Cargo.toml").write_text(updated_manifest, encoding="utf-8")
    if not apply:
        print("Preview only. No working source files changed. Re-run with --apply to create a candidate branch.")
        return
    git("switch", "-c", f"codex/engine-{target[:12]}")
    CANDIDATE.write_text(json.dumps(candidate, indent=2), encoding="utf-8")
    patch = git("diff", "--binary", "--no-renames", base, target, "--", *policy["engine_paths"])
    if patch:
        result = subprocess.run(["git", "apply", "--3way", "--index", "--whitespace=nowarn"], cwd=ROOT, input=patch)
        if result.returncode:
            verify_candidate(policy, candidate)
            raise ValueError("Resolve the core conflicts, review target/engine-sync/Cargo.toml, then run --finalize. UI files were preserved.")
    (ROOT / "Cargo.toml").write_text(updated_manifest, encoding="utf-8")
    finalize(policy)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--ref", help="Upstream tag, branch or commit")
    parser.add_argument("--apply", action="store_true", help="Prepare, build and test a candidate branch")
    parser.add_argument("--finalize", action="store_true", help="Validate a manually reconciled candidate")
    args = parser.parse_args()
    policy = json.loads((ROOT / "engine-policy.json").read_text(encoding="utf-8"))
    if args.finalize:
        finalize(policy)
    elif args.ref:
        prepare(args.ref, args.apply, policy)
    else:
        parser.error("Use --ref TAG or --finalize")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, subprocess.CalledProcessError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
