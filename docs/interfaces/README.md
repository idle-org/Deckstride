# Persistent interfaces

The foundation writes no user settings, project files or exports. Presentation state
is in memory only: page indices are zero-based and blanking retains the selected
page. No file compatibility promise exists yet.

Before introducing persistence, document its location, version, fields, units,
precision, validation, omissions and round trips here. Add versioned examples and
meaningful tests, and provide migration or explicit unsupported-version handling.
Source PDFs always remain unchanged.

[Extraction cache requirements](extraction-cache.md) describe the authorized
pre-rendered image/`extraction.json` direction; its schema is not implemented yet.
