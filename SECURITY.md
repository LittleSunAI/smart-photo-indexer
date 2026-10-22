# Reporting a security issue

Do not publish API keys, personal images, database files, signed result URLs or full user profiles in an issue. A useful first report includes the version, Windows version, affected operation, expected behavior and a minimal reproduction using non-sensitive sample files.

If the repository's GitHub **Security → Advisories → Report a vulnerability** channel is enabled, use it for a private report. If it is unavailable, open a minimal issue asking for a private contact without including vulnerability details or secrets. A dedicated private reporting contact has not yet been published.

The prepared public test entry point never reads the current user's stored cloud keys or calls paid services. Dependency checks and notices are part of release preparation; they are not a guarantee that the application has no security defects.
