#!/usr/bin/env python3
"""
Doctrack Vault Evaluator

Reads a .doctrack/ vault and produces a benchmark JSON with coverage,
graph density, content quality, and issue detection.

Usage:
    python evaluate_vault.py /path/to/project                    # finds .doctrack/ automatically
    python evaluate_vault.py /path/to/.doctrack                  # direct vault path
    python evaluate_vault.py /path/to/project -o report.json     # custom output path
    python evaluate_vault.py /path/to/project --print            # print to stdout instead of file
    python evaluate_vault.py /path/to/project --source src/      # include file registry coverage check
    python evaluate_vault.py /path/to/project --compare prev.json  # compare against previous benchmark
"""

import argparse
import json
import os
import re
import subprocess
import sys
from collections import defaultdict
from datetime import datetime
from pathlib import Path

try:
    import yaml
except ImportError:
    print("PyYAML not found. Install with: pip install pyyaml", file=sys.stderr)
    sys.exit(1)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def parse_frontmatter(content: str) -> tuple[dict, str]:
    """Extract YAML frontmatter and body from a markdown file."""
    if not content.startswith("---"):
        return {}, content
    end = content.find("---", 3)
    if end == -1:
        return {}, content
    try:
        fm = yaml.safe_load(content[3:end]) or {}
    except yaml.YAMLError:
        fm = {}
    body = content[end + 3:].strip()
    return fm, body


def find_vault(path: str) -> Path:
    """Find the .doctrack vault directory."""
    p = Path(path)
    if p.name == ".doctrack" and p.is_dir():
        return p
    doctrack = p / ".doctrack"
    if doctrack.is_dir():
        return doctrack
    print(f"Error: No .doctrack/ vault found at {path}", file=sys.stderr)
    sys.exit(1)


def find_project_root(vault: Path) -> Path:
    """Infer the project root from the vault path."""
    if vault.name == ".doctrack":
        return vault.parent
    return vault.parent


def load_notes(vault: Path) -> list[dict]:
    """Load all markdown notes from the vault."""
    notes = []
    for md in vault.rglob("*.md"):
        if ".obsidian" in md.parts:
            continue
        rel = md.relative_to(vault)
        content = md.read_text(encoding="utf-8", errors="replace")
        fm, body = parse_frontmatter(content)
        body_lines = [l for l in body.split("\n") if l.strip()]
        notes.append({
            "path": str(rel),
            "frontmatter": fm,
            "body": body,
            "full_content": content,
            "size": len(content),
            "body_lines": len(body_lines),
        })
    return notes


def classify_note(note: dict) -> str:
    """Determine note type from path and frontmatter."""
    path = note["path"]
    fm_type = note["frontmatter"].get("type", "")
    if path == "_project.md" or path.endswith("_package.md"):
        return "index"
    for prefix, ntype in [
        ("features/", "feature"), ("components/", "component"),
        ("concepts/", "concept"), ("decisions/", "decision"),
        ("interfaces/", "interface"), ("guides/", "guide"),
        ("specs/", "spec"), ("references/", "reference"),
    ]:
        if path.startswith(prefix):
            return ntype
    if path.startswith("packages/"):
        parts = path.split("/")
        if len(parts) >= 3:
            subdir = parts[2]
            type_map = {
                "features": "feature", "components": "component",
                "concepts": "concept", "decisions": "decision",
                "interfaces": "interface", "guides": "guide",
                "specs": "spec", "references": "reference",
            }
            if subdir in type_map:
                return type_map[subdir]
    return fm_type or "unknown"


def extract_wikilinks(body: str, exclude_mermaid: bool = True) -> list[str]:
    """Extract all [[wikilinks]] from markdown body."""
    text = body
    if exclude_mermaid:
        text = re.sub(r"```mermaid.*?```", "", text, flags=re.DOTALL)
    return re.findall(r"\[\[([^\]]+)\]\]", text)


def normalize_link_target(link: str) -> str:
    """Normalize a wikilink target to a comparable path."""
    target = link.split("|")[0].strip()
    if not target.endswith(".md"):
        target += ".md"
    return target


# ---------------------------------------------------------------------------
# Analysis functions
# ---------------------------------------------------------------------------

def analyze_graph(notes: list[dict]) -> dict:
    """Full graph analysis: density, orphans, hubs, reciprocity."""
    # Build adjacency lists
    outgoing = defaultdict(set)  # note path -> set of target paths
    incoming = defaultdict(set)  # note path -> set of source paths
    all_note_paths = set()
    all_note_stems = {}

    for note in notes:
        path = note["path"]
        all_note_paths.add(path)
        stem = Path(path).stem
        path_no_ext = path.replace(".md", "")
        all_note_stems[stem] = path
        all_note_stems[path_no_ext] = path

    total_links = 0
    cross_type = 0
    type_dirs = {"features", "components", "concepts", "decisions",
                 "interfaces", "guides", "specs", "references", "packages"}

    for note in notes:
        src = note["path"]
        src_dir = src.split("/")[0] if "/" in src else ""
        links = extract_wikilinks(note["body"])
        for link in links:
            total_links += 1
            target = link.split("|")[0].strip()
            # Resolve target to a note path
            resolved = None
            candidates = [
                target + ".md",
                target,
            ]
            for c in candidates:
                if c in all_note_paths:
                    resolved = c
                    break
            if not resolved and target in all_note_stems:
                resolved = all_note_stems[target]
            if resolved:
                outgoing[src].add(resolved)
                incoming[resolved].add(src)

            # Cross-type check
            target_dir = target.split("/")[0] if "/" in target else ""
            if target_dir in type_dirs and target_dir != src_dir:
                cross_type += 1

    # Orphans: no outgoing AND no incoming
    orphans = [p for p in all_note_paths
               if len(outgoing.get(p, set())) == 0
               and len(incoming.get(p, set())) == 0]

    # Bidirectional links
    bidirectional = 0
    total_directed_edges = 0
    for src, targets in outgoing.items():
        for tgt in targets:
            total_directed_edges += 1
            if src in outgoing.get(tgt, set()):
                bidirectional += 1
    bidirectional_ratio = round(bidirectional / total_directed_edges, 2) if total_directed_edges > 0 else 0

    # Most / least connected
    connectivity = {}
    for note in notes:
        p = note["path"]
        out_count = len(outgoing.get(p, set()))
        in_count = len(incoming.get(p, set()))
        connectivity[p] = {"outgoing": out_count, "incoming": in_count, "total": out_count + in_count}

    sorted_by_total = sorted(connectivity.items(), key=lambda x: -x[1]["total"])
    most_connected = [{"path": p, **c} for p, c in sorted_by_total[:10]]
    # Least connected excluding orphans
    non_orphan = [(p, c) for p, c in sorted_by_total if c["total"] > 0]
    least_connected = [{"path": p, **c} for p, c in reversed(non_orphan[-10:])]

    n = len(notes)
    avg_links = round(total_links / n, 1) if n > 0 else 0

    return {
        "total_wikilinks": total_links,
        "links_per_note": avg_links,
        "orphan_notes": len(orphans),
        "orphan_paths": sorted(orphans),
        "cross_type_links": cross_type,
        "cross_type_ratio": round(cross_type / total_links, 2) if total_links > 0 else 0,
        "bidirectional_links": bidirectional,
        "bidirectional_ratio": bidirectional_ratio,
        "most_connected": most_connected,
        "least_connected": least_connected,
    }


def analyze_content_quality(notes: list[dict]) -> dict:
    """Content depth analysis: sizes, stubs, diagrams."""
    sizes = [n["size"] for n in notes]
    avg_size = round(sum(sizes) / len(sizes)) if sizes else 0

    stub_threshold = 500  # bytes
    stub_line_threshold = 5
    stubs = [
        {"path": n["path"], "size": n["size"], "lines": n["body_lines"]}
        for n in notes
        if n["size"] < stub_threshold or n["body_lines"] < stub_line_threshold
    ]

    # Mermaid diagram coverage by type
    notes_with_mermaid = defaultdict(int)
    notes_total_by_type = defaultdict(int)
    total_diagrams = 0
    internal_link_nodes = 0
    wikilinks_in_mermaid = 0
    hex_colors_in_mermaid = 0

    for note in notes:
        ntype = note.get("_type", "unknown")
        notes_total_by_type[ntype] += 1
        mermaid_blocks = re.findall(r"```mermaid(.*?)```", note["full_content"], re.DOTALL)
        if mermaid_blocks:
            notes_with_mermaid[ntype] += 1
            total_diagrams += len(mermaid_blocks)
            for block in mermaid_blocks:
                internal_link_nodes += len(re.findall(r"internal-link", block))
                wikilinks_in_mermaid += len(re.findall(r"\[\[", block))
                hex_colors_in_mermaid += len(re.findall(r"#[0-9a-fA-F]{3,8}", block))

    diagram_coverage = {}
    for ntype in ["feature", "concept", "interface", "decision"]:
        total = notes_total_by_type.get(ntype, 0)
        with_diagram = notes_with_mermaid.get(ntype, 0)
        diagram_coverage[ntype] = {
            "total": total,
            "with_diagram": with_diagram,
            "percentage": round(with_diagram / total * 100) if total > 0 else 0,
        }

    return {
        "average_note_size_bytes": avg_size,
        "stubs": {
            "count": len(stubs),
            "threshold_bytes": stub_threshold,
            "threshold_lines": stub_line_threshold,
            "stub_notes": stubs[:15],
        },
        "mermaid": {
            "total_diagrams": total_diagrams,
            "internal_link_nodes": internal_link_nodes,
            "wikilinks_in_mermaid": wikilinks_in_mermaid,
            "hex_colors_in_mermaid": hex_colors_in_mermaid,
            "diagram_coverage": diagram_coverage,
        },
    }


def analyze_tags(notes: list[dict]) -> dict:
    """Tag taxonomy analysis."""
    all_stray = []
    notes_complete = 0
    notes_missing = []

    for note in notes:
        if note.get("_type") in ("index", "unknown"):
            continue
        tags = note["frontmatter"].get("tags", [])
        if not isinstance(tags, list):
            tags = []
        doctrack = [t for t in tags if t.startswith("doctrack/")]
        stray = [t for t in tags if not t.startswith("doctrack/")]
        all_stray.extend(stray)
        has_type = any(t.startswith("doctrack/type/") for t in doctrack)
        has_status = any(t.startswith("doctrack/status/") for t in doctrack)
        has_audience = any(t.startswith("doctrack/audience/") for t in doctrack)
        if has_type and has_status and has_audience:
            notes_complete += 1
        else:
            notes_missing.append({
                "path": note["path"],
                "missing": [x for x, present in
                            [("type", has_type), ("status", has_status), ("audience", has_audience)]
                            if not present],
            })

    stray_unique = sorted(set(all_stray))
    return {
        "notes_with_complete_taxonomy": notes_complete,
        "notes_missing_taxonomy": len(notes_missing),
        "missing_taxonomy_details": notes_missing[:15],
        "stray_tag_count": len(stray_unique),
        "stray_tags": stray_unique[:30],
    }


def analyze_frontmatter(notes: list[dict]) -> dict:
    """Frontmatter field completeness."""
    with_version = 0
    with_last_updated = 0
    with_files = 0
    with_feature_field = 0
    components_missing_feature = []
    features_missing_files = []

    for note in notes:
        fm = note["frontmatter"]
        if fm.get("doctrack_version"):
            with_version += 1
        if fm.get("last_updated"):
            with_last_updated += 1
        if fm.get("files"):
            with_files += 1
        if fm.get("feature"):
            with_feature_field += 1
        if note.get("_type") == "component" and not fm.get("feature"):
            components_missing_feature.append(note["path"])
        if note.get("_type") == "feature" and not fm.get("files"):
            features_missing_files.append(note["path"])

    return {
        "notes_with_doctrack_version": with_version,
        "notes_with_last_updated": with_last_updated,
        "notes_with_files_field": with_files,
        "notes_with_feature_field": with_feature_field,
        "components_missing_feature_ref": components_missing_feature[:10],
        "features_missing_files_list": features_missing_files[:10],
    }


def analyze_file_registry(notes: list[dict], project_root: Path, source_dirs: list[str]) -> dict:
    """Compare source files on disk against the file registry in feature/component frontmatter."""
    if not source_dirs:
        return {"enabled": False}

    # Collect all source files from disk
    source_extensions = {".java", ".kt", ".py", ".js", ".ts", ".tsx", ".jsx",
                         ".go", ".rs", ".swift", ".c", ".cpp", ".h", ".cs", ".rb"}
    disk_files = set()
    for sdir in source_dirs:
        src_path = project_root / sdir
        if not src_path.exists():
            # Try relative to project root using glob
            for ext in source_extensions:
                for f in project_root.rglob(f"*{ext}"):
                    if ".doctrack" not in f.parts and ".git" not in f.parts:
                        rel = str(f.relative_to(project_root))
                        disk_files.add(rel)
            break
        for f in src_path.rglob("*"):
            if f.is_file() and f.suffix in source_extensions:
                rel = str(f.relative_to(project_root))
                disk_files.add(rel)

    # Collect all files referenced in frontmatter
    registered_files = set()
    for note in notes:
        files = note["frontmatter"].get("files", [])
        if isinstance(files, list):
            for f in files:
                if isinstance(f, str):
                    registered_files.add(f)

    # Compare
    mapped = disk_files & registered_files
    unmapped = disk_files - registered_files

    coverage = round(len(mapped) / len(disk_files) * 100, 1) if disk_files else 0

    return {
        "enabled": True,
        "disk_source_files": len(disk_files),
        "registered_files": len(registered_files),
        "mapped_files": len(mapped),
        "unmapped_files": len(unmapped),
        "coverage_percentage": coverage,
        "unmapped_sample": sorted(list(unmapped))[:20],
    }


def analyze_component_depth(notes: list[dict], project_root: Path) -> dict:
    """Compare source file counts per module against component counts."""
    by_type = defaultdict(list)
    for note in notes:
        by_type[note.get("_type", "unknown")].append(note)

    components_by_module = defaultdict(int)
    for note in by_type.get("component", []):
        parts = note["path"].split("/")
        if len(parts) >= 2:
            components_by_module[parts[1]] += 1

    source_extensions = {".java", ".kt", ".py", ".js", ".ts", ".go", ".rs", ".swift"}
    module_analysis = []

    for feature in by_type.get("feature", []):
        module_name = Path(feature["path"]).stem
        comp_count = components_by_module.get(module_name, 0)

        # Try to count source files for this module
        file_count = 0
        module_dir = project_root / module_name
        if module_dir.is_dir():
            for f in module_dir.rglob("*"):
                if f.is_file() and f.suffix in source_extensions:
                    file_count += 1

        # Expected components based on skill guidelines
        if file_count <= 2:
            expected_min = 0
        elif file_count <= 10:
            expected_min = 2
        elif file_count <= 30:
            expected_min = 5
        elif file_count <= 100:
            expected_min = 15
        else:
            expected_min = 30

        below_expected = comp_count < expected_min and file_count > 2
        module_analysis.append({
            "module": module_name,
            "source_files": file_count,
            "components": comp_count,
            "expected_min": expected_min,
            "below_expected": below_expected,
        })

    module_analysis.sort(key=lambda x: -x["source_files"])
    flagged = [m for m in module_analysis if m["below_expected"]]

    return {
        "modules": module_analysis,
        "flagged_modules": flagged,
        "flagged_count": len(flagged),
    }


def analyze_staleness(notes: list[dict]) -> dict:
    """Find notes with old last_updated dates."""
    now = datetime.now()
    stale = []
    for note in notes:
        lu = note["frontmatter"].get("last_updated")
        if not lu:
            continue
        try:
            if isinstance(lu, str):
                dt = datetime.fromisoformat(lu.replace("Z", "+00:00").replace("'", ""))
            elif hasattr(lu, "isoformat"):
                dt = datetime(lu.year, lu.month, lu.day)
            else:
                continue
            age_days = (now - dt.replace(tzinfo=None)).days
            if age_days > 30:
                stale.append({"path": note["path"], "last_updated": str(lu), "age_days": age_days})
        except (ValueError, TypeError):
            continue

    stale.sort(key=lambda x: -x["age_days"])
    return {
        "stale_notes_over_30d": len(stale),
        "stale_notes": stale[:15],
    }


# ---------------------------------------------------------------------------
# Scoring
# ---------------------------------------------------------------------------

def compute_scores(report: dict) -> dict:
    """Compute 0-100 scores across dimensions."""
    scores = {}
    cov = report["coverage"]

    # Coverage: are all node types populated?
    coverage_checks = [
        cov["features"] > 0,
        cov["components"] > 0,
        cov["concepts"] >= 3,
        cov["decisions"] >= 3,
        cov["interfaces"] >= 2,
        cov["guides"] > 0,
        cov["references"] > 0,
    ]
    scores["coverage"] = round(sum(coverage_checks) / len(coverage_checks) * 100)

    # Density: how interconnected is the graph?
    density = report["graph_density"]
    density_checks = [
        density["links_per_note"] >= 3.0,
        density["links_per_note"] >= 5.0,
        density["orphan_notes"] == 0,
        density["cross_type_ratio"] >= 0.2,
        density["cross_type_ratio"] >= 0.4,
        density["bidirectional_ratio"] >= 0.1,
        density["bidirectional_ratio"] >= 0.3,
    ]
    scores["density"] = round(sum(density_checks) / len(density_checks) * 100)

    # Quality: tags, mermaid, checkpoint
    mermaid = report["content_quality"]["mermaid"]
    tags = report["tags"]
    quality_checks = [
        mermaid["wikilinks_in_mermaid"] == 0,
        mermaid["hex_colors_in_mermaid"] == 0,
        tags["stray_tag_count"] == 0,
        tags["notes_missing_taxonomy"] == 0,
        report["checkpoint"]["current_phase"] == "complete",
        report["content_quality"]["stubs"]["count"] == 0,
    ]
    scores["quality"] = round(sum(quality_checks) / len(quality_checks) * 100)

    # Component depth
    if cov["features"] > 0:
        ratio = cov["components"] / cov["features"]
        if ratio >= 8:
            scores["component_depth"] = 100
        elif ratio >= 5:
            scores["component_depth"] = 80
        elif ratio >= 3:
            scores["component_depth"] = 60
        elif ratio >= 1:
            scores["component_depth"] = 40
        else:
            scores["component_depth"] = 20
    else:
        scores["component_depth"] = 0

    # Diagram coverage: % of features+concepts with mermaid
    diag = report["content_quality"]["mermaid"]["diagram_coverage"]
    diag_pcts = [diag.get(t, {}).get("percentage", 0) for t in ["feature", "concept"]]
    avg_diag = sum(diag_pcts) / len(diag_pcts) if diag_pcts else 0
    scores["diagram_coverage"] = round(avg_diag)

    # Frontmatter completeness
    fm = report["frontmatter_quality"]
    total = report["vault_stats"]["total_notes"]
    fm_checks = [
        fm["notes_with_last_updated"] / total >= 0.8 if total > 0 else False,
        fm["notes_with_files_field"] / total >= 0.5 if total > 0 else False,
        len(fm["components_missing_feature_ref"]) == 0,
        len(fm["features_missing_files_list"]) == 0,
    ]
    scores["frontmatter"] = round(sum(fm_checks) / len(fm_checks) * 100)

    # Overall (weighted)
    scores["overall"] = round(
        scores["coverage"] * 0.15 +
        scores["density"] * 0.25 +
        scores["quality"] * 0.20 +
        scores["component_depth"] * 0.15 +
        scores["diagram_coverage"] * 0.10 +
        scores["frontmatter"] * 0.15
    )

    return scores


# ---------------------------------------------------------------------------
# Comparison
# ---------------------------------------------------------------------------

def compare_reports(current: dict, previous: dict) -> dict:
    """Generate a delta between current and previous benchmarks."""
    def delta(cur, prev, key_path):
        keys = key_path.split(".")
        c = cur
        p = prev
        for k in keys:
            c = c.get(k, 0) if isinstance(c, dict) else 0
            p = p.get(k, 0) if isinstance(p, dict) else 0
        if isinstance(c, (int, float)) and isinstance(p, (int, float)):
            diff = c - p
            sign = "+" if diff > 0 else ""
            return {"current": c, "previous": p, "delta": f"{sign}{diff}"}
        return {"current": c, "previous": p, "delta": "n/a"}

    return {
        "total_notes": delta(current, previous, "vault_stats.total_notes"),
        "vault_size": delta(current, previous, "vault_stats.vault_size_bytes"),
        "features": delta(current, previous, "coverage.features"),
        "components": delta(current, previous, "coverage.components"),
        "concepts": delta(current, previous, "coverage.concepts"),
        "decisions": delta(current, previous, "coverage.decisions"),
        "interfaces": delta(current, previous, "coverage.interfaces"),
        "wikilinks": delta(current, previous, "graph_density.total_wikilinks"),
        "links_per_note": delta(current, previous, "graph_density.links_per_note"),
        "orphans": delta(current, previous, "graph_density.orphan_notes"),
        "cross_type_links": delta(current, previous, "graph_density.cross_type_links"),
        "bidirectional_ratio": delta(current, previous, "graph_density.bidirectional_ratio"),
        "stray_tags": delta(current, previous, "tags.stray_tag_count"),
        "overall_score": delta(current, previous, "scores.overall"),
    }


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def evaluate(vault: Path, project_root: Path, source_dirs: list[str]) -> dict:
    """Run full vault evaluation."""
    notes = load_notes(vault)

    # Classify
    by_type = defaultdict(list)
    for note in notes:
        ntype = classify_note(note)
        note["_type"] = ntype
        by_type[ntype].append(note)

    # Components per module
    components_by_module = defaultdict(int)
    for note in by_type["component"]:
        parts = note["path"].split("/")
        if len(parts) >= 2:
            components_by_module[parts[1]] += 1

    # Checkpoint
    project_note = next((n for n in notes if n["path"] == "_project.md"), None)
    checkpoint_phase = "unknown"
    if project_note:
        match = re.search(r"Current phase:\s*\*?\*?(\S+)\*?\*?", project_note["body"])
        if match:
            checkpoint_phase = match.group(1).strip("*")

    # Run all analyses
    graph = analyze_graph(notes)
    content = analyze_content_quality(notes)
    tags = analyze_tags(notes)
    frontmatter = analyze_frontmatter(notes)
    file_reg = analyze_file_registry(notes, project_root, source_dirs)
    comp_depth = analyze_component_depth(notes, project_root)
    staleness = analyze_staleness(notes)

    report = {
        "vault_path": str(vault),
        "project_root": str(project_root),
        "evaluated_at": datetime.now().isoformat(),
        "vault_stats": {
            "total_notes": len(notes),
            "total_folders": len(set(str(Path(n["path"]).parent) for n in notes)),
            "vault_size_bytes": sum(n["size"] for n in notes),
        },
        "coverage": {
            "features": len(by_type["feature"]),
            "components": len(by_type["component"]),
            "concepts": len(by_type["concept"]),
            "decisions": len(by_type["decision"]),
            "interfaces": len(by_type["interface"]),
            "guides": len(by_type["guide"]),
            "references": len(by_type["reference"]),
            "specs": len(by_type["spec"]),
            "unknown": len(by_type["unknown"]),
        },
        "components_per_module": dict(sorted(
            components_by_module.items(), key=lambda x: -x[1]
        )),
        "graph_density": graph,
        "content_quality": content,
        "tags": tags,
        "frontmatter_quality": frontmatter,
        "file_registry": file_reg,
        "component_depth": comp_depth,
        "staleness": staleness,
        "checkpoint": {"current_phase": checkpoint_phase},
        "scores": {},
    }

    report["scores"] = compute_scores(report)
    return report


def print_summary(report: dict, comparison: dict = None):
    """Print human-readable summary to stderr."""
    s = report["scores"]
    g = report["graph_density"]
    c = report["coverage"]
    q = report["content_quality"]
    t = report["tags"]

    print(f"\n{'='*60}", file=sys.stderr)
    print(f"  Doctrack Vault Evaluation", file=sys.stderr)
    print(f"{'='*60}", file=sys.stderr)
    print(f"  Notes:        {report['vault_stats']['total_notes']}", file=sys.stderr)
    print(f"  Vault size:   {report['vault_stats']['vault_size_bytes'] / 1024:.0f} KB", file=sys.stderr)
    print(f"  Phase:        {report['checkpoint']['current_phase']}", file=sys.stderr)
    print(f"{'─'*60}", file=sys.stderr)
    print(f"  Features:     {c['features']:>4}    Components:  {c['components']:>4}", file=sys.stderr)
    print(f"  Concepts:     {c['concepts']:>4}    Decisions:   {c['decisions']:>4}", file=sys.stderr)
    print(f"  Interfaces:   {c['interfaces']:>4}    Guides:      {c['guides']:>4}", file=sys.stderr)
    print(f"  References:   {c['references']:>4}    Specs:       {c['specs']:>4}", file=sys.stderr)
    print(f"{'─'*60}", file=sys.stderr)
    print(f"  Wikilinks:    {g['total_wikilinks']} ({g['links_per_note']}/note)", file=sys.stderr)
    print(f"  Cross-type:   {g['cross_type_links']} ({g['cross_type_ratio']:.0%})", file=sys.stderr)
    print(f"  Bidirectional:{g['bidirectional_links']} ({g['bidirectional_ratio']:.0%})", file=sys.stderr)
    print(f"  Orphans:      {g['orphan_notes']}", file=sys.stderr)
    print(f"  Diagrams:     {q['mermaid']['total_diagrams']} ({q['mermaid']['internal_link_nodes']} clickable nodes)", file=sys.stderr)
    print(f"  Stubs:        {q['stubs']['count']}", file=sys.stderr)
    print(f"  Stray tags:   {t['stray_tag_count']}", file=sys.stderr)
    print(f"  Missing taxonomy: {t['notes_missing_taxonomy']}", file=sys.stderr)

    if report["file_registry"]["enabled"]:
        fr = report["file_registry"]
        print(f"  File registry:{fr['coverage_percentage']}% ({fr['mapped_files']}/{fr['disk_source_files']})", file=sys.stderr)

    if report["component_depth"]["flagged_count"] > 0:
        print(f"  Undercomp:    {report['component_depth']['flagged_count']} modules below expected", file=sys.stderr)

    print(f"{'='*60}", file=sys.stderr)
    print(f"  Coverage:          {s['coverage']:>3}%", file=sys.stderr)
    print(f"  Graph density:     {s['density']:>3}%", file=sys.stderr)
    print(f"  Quality:           {s['quality']:>3}%", file=sys.stderr)
    print(f"  Component depth:   {s['component_depth']:>3}%", file=sys.stderr)
    print(f"  Diagram coverage:  {s['diagram_coverage']:>3}%", file=sys.stderr)
    print(f"  Frontmatter:       {s['frontmatter']:>3}%", file=sys.stderr)
    print(f"  {'─'*40}", file=sys.stderr)
    print(f"  OVERALL:           {s['overall']:>3}%", file=sys.stderr)
    print(f"{'='*60}", file=sys.stderr)

    if comparison:
        print(f"\n  Comparison with previous:", file=sys.stderr)
        for k, v in comparison.items():
            if v["delta"] != "n/a" and v["delta"] != "+0" and v["delta"] != "+0.0":
                print(f"    {k}: {v['previous']} → {v['current']} ({v['delta']})", file=sys.stderr)
        print(f"{'='*60}", file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(description="Evaluate a doctrack vault")
    parser.add_argument("path", help="Path to project or .doctrack/ vault")
    parser.add_argument("-o", "--output", help="Output JSON path")
    parser.add_argument("--print", action="store_true", help="Print JSON to stdout")
    parser.add_argument("--source", nargs="*", default=[], help="Source dirs for file registry check (e.g., src/ lib/)")
    parser.add_argument("--compare", help="Previous benchmark JSON to compare against")
    args = parser.parse_args()

    vault = find_vault(args.path)
    project_root = find_project_root(vault)
    print(f"Evaluating vault: {vault}", file=sys.stderr)
    print(f"Project root: {project_root}", file=sys.stderr)

    source_dirs = args.source if args.source else []
    # Auto-detect: if no source dirs specified, scan whole project
    if not source_dirs:
        source_dirs = ["."]

    report = evaluate(vault, project_root, source_dirs)

    # Comparison
    comparison = None
    if args.compare:
        prev_path = Path(args.compare)
        if prev_path.exists():
            previous = json.loads(prev_path.read_text())
            comparison = compare_reports(report, previous)
            report["comparison"] = comparison

    output = json.dumps(report, indent=2)

    if args.print:
        print(output)
    else:
        if args.output:
            out_path = Path(args.output)
        else:
            out_path = Path("benchmarks") / f"eval-{datetime.now().strftime('%Y%m%d-%H%M%S')}.json"
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(output)
        print(f"\nReport saved to: {out_path}", file=sys.stderr)

    print_summary(report, comparison)


if __name__ == "__main__":
    main()
