# semver-tidy

Version strings that come from changelogs, git tags, or hand-edited config
files are rarely clean semver. You get `v1.2.3`, `1.02.3`, ` 1.2.3 `, mixed
`V` prefixes, and the occasional typo that only shows up when some other
tool chokes on it. `semver-tidy` reads version strings one per line and
prints the normalized `major.minor.patch[-prerelease][+build]` form, or a
precise error if a line isn't a version at all.

## Usage

From a file, one version per line:

```
$ cat versions.txt
v1.2.3
1.02.3
V2.0.0-Alpha.01
1.0.0+build.007

$ semver-tidy versions.txt
1.2.3
1.2.3
2.0.0-Alpha.1
1.0.0+build.007
```

Notes on that output: `1.02.3` loses its extra zero, `01` in a numeric
pre-release identifier becomes `1` (numeric pre-release identifiers compare
by value, so the zero is just noise), and `build.007` is left untouched,
since build metadata is opaque and not supposed to be reinterpreted.

Or pipe input through stdin:

```
$ echo "v1.2.3" | semver-tidy
1.2.3
```

Pass `--check` to validate without printing the normalized output - useful
in scripts that only care whether every line parsed:

```
$ semver-tidy --check versions.txt; echo "exit: $?"
exit: 0

$ printf '1.2.3\n1.2\n' | semver-tidy --check
error: expected '.' followed by the patch version, found end of line
  --> line 2, column 4
  |
2 | 1.2
  |    ^
```

## Error messages

When a line isn't parseable, the point of failure is reported with the
line and column where the parser gave up, plus the offending text:

```
$ printf '1.2.3\n1.2\n1.2.3-\n' | semver-tidy
1.2.3
error: expected '.' followed by the patch version, found end of line
  --> line 2, column 4
  |
2 | 1.2
  |    ^
error: expected a pre-release identifier, found end of line
  --> line 3, column 7
  |
3 | 1.2.3-
  |       ^
```

Lines that parse successfully still print to stdout even when later lines
fail, so a single bad entry in a large file doesn't hide the good ones.
Errors go to stderr, and the process exits non-zero if any line failed.

Pass `--strip-build` to drop any `+build` suffix from the output, useful
when two versions that differ only in build metadata should be treated as
the same string downstream:

```
$ echo "1.0.0+build.007" | semver-tidy --strip-build
1.0.0
```

`--strip-build` and `--check` can be combined; stripping only changes what
would have been printed, not whether a line is considered valid.

## What counts as valid

- Three dot-separated numeric components (`major.minor.patch`), each
  normalized by stripping leading zeros.
- An optional leading `v` or `V`, which is dropped.
- An optional `-` followed by dot-separated pre-release identifiers
  (alphanumeric and hyphens). Purely numeric identifiers have their
  leading zeros stripped; anything else is left as-is.
- An optional `+` followed by dot-separated build-metadata identifiers,
  preserved verbatim.
- Surrounding whitespace, which is trimmed.

Anything else - a missing component, a stray character, an empty
identifier between two dots - is an error naming exactly what was
expected and where.

## Status

The parser and CLI work end to end and are covered by integration test
suites (`tests/parser.rs` for parsing, `tests/cli.rs` for the binary,
`tests/compare.rs` for precedence comparison). The identifier character
set already matches the semver 2.0.0 grammar (`[0-9A-Za-z-]`, non-empty);
the one deliberate deviation from a strict reading is that numeric
pre-release identifiers with leading zeros are normalized rather than
rejected, consistent with how this tool treats the version core.

The library exposes `parse_line` for callers that need the structured
`Version` rather than just the canonical string, and `Version::compare_precedence`
for ordering two versions by semver rules (build metadata is ignored, as
the spec requires). There's no CLI flag for comparison yet - it's
library-only for now.

## License

MIT, see [LICENSE](LICENSE).
