#!/bin/sh
# trace.sh — requirements traceability report for the R10 convention, run as a
# CI gate (ci.sh). Plain sh + awk + find only; no other dependencies.
# Adapted from experiments/exp06-traceability/trace.sh; scans every workspace
# member's src/ and tests/ trees.
#
# For every requirement REQ-NNN it reports:
#   defined at    the `/// REQ-NNN: ...` doc line and the trait it documents
#   satisfied by  items carrying a `/// Satisfies: REQ-NNN` doc tag, plus every
#                 code line that uses the requirement trait as a bound
#   verified by   tests carrying a `/// Verifies: REQ-NNN` doc tag
# and warns about (a) requirements with no verifying test, (b) Verifies/
# Satisfies tags naming an unknown requirement, and (c) near-miss tags.
# Any warning fails CI (exit 1).
#
# When the workspace defines NO requirements at all, that is not a violation:
# model-core is infrastructure and R10 requirement definitions belong to the
# downstream modelling crates. The report then says so and exits 0 (unless
# there are orphan tags or near-miss warnings, which still fail).
#
# Convention enforced (R10, F-021):
#   * tags live in doc comments: exactly three slashes (`///`), one tag per
#     line, case-sensitive `Verifies:` / `Satisfies:`, IDs as REQ-NNN
#     separated by commas; the tag line sits in the doc block directly above
#     the item (attributes such as #[test] may intervene)
#   * a requirement definition is a doc line starting `/// REQ-NNN:` directly
#     above `trait ReqNNN...` (the requirement! macro keeps both lines
#     literal at its invocation site)
#   * `//`, `////`, `//!`, `/* */` comments and string literals never count.

set -eu
cd "$(dirname "$0")"

FILES=$(find . -path '*/target' -prune -o -type f -name '*.rs' \
        \( -path '*/src/*' -o -path '*/tests/*' \) -print | sort)
[ -n "$FILES" ] || { echo "no .rs files under any member's src/ or tests/" >&2; exit 2; }

# shellcheck disable=SC2086
awk '
# ---- helpers ---------------------------------------------------------------
function is_doc(line) {          # exactly three leading slashes
    return match(line, /^[ \t]*\/\/\//) && substr(line, RSTART + RLENGTH, 1) != "/"
}
function doc_body(line) { sub(/^[ \t]*\/\/\//, "", line); return line }
function extract_ids(s, out,    n) {
    n = 0
    while (match(s, /REQ-[0-9][0-9][0-9]/)) {
        out[++n] = substr(s, RSTART, RLENGTH)
        s = substr(s, RSTART + RLENGTH)
    }
    return n
}
function item_name(line) {
    if (match(line, /fn [A-Za-z_][A-Za-z0-9_]*/))     return substr(line, RSTART + 3, RLENGTH - 3)
    if (match(line, /struct [A-Za-z_][A-Za-z0-9_]*/)) return substr(line, RSTART + 7, RLENGTH - 7)
    if (match(line, /trait [A-Za-z_][A-Za-z0-9_]*/))  return substr(line, RSTART + 6, RLENGTH - 6)
    if (match(line, /enum [A-Za-z_][A-Za-z0-9_]*/))   return substr(line, RSTART + 5, RLENGTH - 5)
    if (match(line, /type [A-Za-z_][A-Za-z0-9_]*/))   return substr(line, RSTART + 5, RLENGTH - 5)
    return ""
}
function trimmed(line) {
    sub(/^[ \t]+/, "", line); sub(/[ \t]+$/, "", line)
    if (length(line) > 66) line = substr(line, 1, 63) "..."
    return line
}

# ---- pass 1: definitions and doc tags --------------------------------------
pass == 1 {
    line = $0
    loc = FILENAME ":" FNR
    if (is_doc(line)) {
        body = doc_body(line)
        if (match(body, /^[ \t]*REQ-[0-9][0-9][0-9]:/)) {
            extract_ids(body, tmp)
            pd_id = tmp[1]; pd_loc = loc
            pd_desc = trimmed(substr(body, index(body, ":") + 1))
        } else if (p = index(body, "Satisfies:")) {
            n = extract_ids(substr(body, p), tmp)
            for (i = 1; i <= n; i++) { ps_ids[++ps_n] = tmp[i]; ps_locs[ps_n] = loc }
        } else if (p = index(body, "Verifies:")) {
            n = extract_ids(substr(body, p), tmp)
            for (i = 1; i <= n; i++) { pv_ids[++pv_n] = tmp[i]; pv_locs[pv_n] = loc }
        } else if (tolower(body) ~ /(verifies|satisfies):/) {
            # near-miss: looks like a tag but is not in canonical form
            near[++near_n] = loc ": " trimmed(body)
        }
        next
    }
    if (line ~ /^[ \t]*#\[/ || line ~ /^[ \t]*$/) next   # attributes, blanks: keep pending tags
    name = item_name(line)
    if (pd_id != "") {
        if (name != "") {
            def_loc[pd_id] = pd_loc; def_name[pd_id] = name; def_desc[pd_id] = pd_desc
            ndef++
        }
        pd_id = ""
    }
    if (ps_n > 0) {
        if (name != "")
            for (i = 1; i <= ps_n; i++) {
                id = ps_ids[i]
                sat[id] = sat[id] sprintf("    satisfied by  %-34s %s  [Satisfies tag]\n", loc, name)
                tag_seen[id] = tag_seen[id] ps_locs[i] " (Satisfies on " name ") "
            }
        ps_n = 0
    }
    if (pv_n > 0) {
        if (name != "")
            for (i = 1; i <= pv_n; i++) {
                id = pv_ids[i]
                ver[id] = ver[id] sprintf("    verified by   %-34s %s\n", loc, name)
                ver_count[id]++
                tag_seen[id] = tag_seen[id] pv_locs[i] " (Verifies on " name ") "
            }
        pv_n = 0
    }
}

# ---- pass 2: uses of the requirement traits (bounds, assertions) -----------
pass == 2 {
    if ($0 ~ /^[ \t]*\/\//) next                          # any comment line
    if ($0 ~ /^[ \t]*use /) next                          # imports are not uses
    for (id in def_name) {
        nm = def_name[id]
        if (index($0, nm) == 0) continue
        if (match($0, "trait[ \t]+" nm)) continue          # the definition itself
        if ($0 ~ /^[ \t]*impl</ && index($0, " for ")) continue  # the blanket impl
        sat[id] = sat[id] sprintf("    satisfied by  %-34s %s  [bound use]\n", \
                                  FILENAME ":" FNR, trimmed($0))
    }
}

# ---- report -----------------------------------------------------------------
END {
    print "Requirements traceability report"
    print "================================"
    if (ndef == 0) {
        print ""
        print "No requirements defined in this workspace."
        print "(model-core is infrastructure: R10 requirement traits belong to the"
        print "downstream modelling crates. This is expected until a pilot crate"
        print "with REQ-NNN definitions joins the workspace.)"
    }
    for (n = 0; n <= 999; n++) {
        id = sprintf("REQ-%03d", n)
        if (!(id in def_loc)) continue
        printf "\n%s  %s\n", id, def_desc[id]
        printf "    defined at    %-34s trait %s\n", def_loc[id], def_name[id]
        printf "%s", (id in sat) ? sat[id] : "    satisfied by  (nothing)\n"
        printf "%s", (id in ver) ? ver[id] : "    verified by   (no test)\n"
    }
    print ""
    warnings = 0
    for (n = 0; n <= 999; n++) {
        id = sprintf("REQ-%03d", n)
        if ((id in def_loc) && !(id in ver_count)) {
            printf "WARNING: %s has no verifying test (defined at %s)\n", id, def_loc[id]
            warnings++
        }
    }
    for (id in tag_seen)
        if (!(id in def_loc)) {
            printf "WARNING: unknown requirement %s tagged at %s\n", id, tag_seen[id]
            warnings++
        }
    for (i = 1; i <= near_n; i++) {
        printf "WARNING: malformed tag (not canonical Verifies:/Satisfies:) at %s\n", near[i]
        warnings++
    }
    if (warnings == 0) {
        if (ndef == 0)
            print "No warnings (and no requirements defined)."
        else
            print "No warnings: every requirement has at least one verifying test."
    }
    exit warnings > 0 ? 1 : 0
}
' pass=1 $FILES pass=2 $FILES
