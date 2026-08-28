# Security

Please report suspected vulnerabilities privately through the GitHub repository
security-advisory form. Do not include credentials, session tokens, private user
data, or exploitable details in a public issue.

This Qt desktop client stores a sanitized JSON document under
`~/.config/happy-wakey/config.json` with mode `0600`. Session tokens currently
live in that file; production builds should move them to the OS keychain.
Shared-auth tokens stay in process memory and are cleared on logout.

`HAPPY_WAKEY_PLATFORM_URL` has no compiled or env default. Numeric IP hosts
are rejected except loopback. Bookmarks accept HTTPS names or loopback HTTP
only. BLE preview commands contain schema, operation UUID, action, and
duration—never tokens, subjects, or owner IDs.
