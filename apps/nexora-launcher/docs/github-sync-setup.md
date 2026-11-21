# GitHub configuration sync setup

NEXORA uses a GitHub App user access token obtained through the OAuth 2.0
Device Authorization Grant. The client does not contain a client secret.

## Register the GitHub App

1. Open GitHub Settings > Developer settings > GitHub Apps > New GitHub App.
2. Disable webhooks unless they are needed later.
3. Enable **Device Flow**.
4. Set the user permission **Gists** to **Read and write**.
5. Keep callback URLs empty because the desktop app uses Device Flow.
6. Copy the app's **Client ID**, not the App ID.

## Build configuration

NEXORA ships with the public Client ID for its GitHub App. The
`NEXORA_GITHUB_CLIENT_ID` environment variable is only needed as a development
override:

```powershell
$env:NEXORA_GITHUB_CLIENT_ID = "your_client_id"
pnpm package
```

For local development:

```powershell
$env:NEXORA_GITHUB_CLIENT_ID = "your_client_id"
pnpm tauri dev
```

The Client ID is not a secret and is embedded in the desktop build.

## Data and security

- Configuration is stored in a Secret Gist named in its description as
  `NEXORA configuration`.
- Secret gists are unlisted, not private. Users must not upload secrets.
- GitHub access and refresh tokens are encrypted at rest with Windows DPAPI.
- Upload and restore are user-triggered. Restoring replaces local settings and
  commands with the selected GitHub Gist.
