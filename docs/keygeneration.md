# APT Repository GPG Key Setup

Stasis APT repository uses GPG signatures to verify package integrity. You need a GPG keypair to sign `.deb` packages published to the `gh-pages` branch.

## 1. Generate GPG Keypair

```bash
gpg --batch --gen-key <<'EOF'
%no-protection
Key-Type: RSA
Key-Length: 4096
Subkey-Type: RSA
Subkey-Length: 4096
Name-Real: Stasis APT Repo
Name-Email: stasis@apt.local
Expire-Date: 0
%commit
EOF
```

## 2. Get Your Key ID

```bash
gpg --list-keys --keyid-format long "Stasis APT Repo"
```

Output:

```
pub   rsa4096/XXXXXXXXXXXXXXXX 2026-08-19 [SCEAR]
      AD91C342DC8C54CC6A87E9493EBB282C8DB8D179
uid                 [ultimate] Stasis APT Repo <stasis@apt.local>
sub   rsa4096/YYYYYYYYYYYYYYYY 2026-08-19 [SEA]
```

Use the 16-character hex ID after `rsa4096/` on the `pub` line (e.g. `XXXXXXXXXXXXXXXX`).

## 3. Export Private Key

```bash
gpg --armor --export-secret-keys YOUR_KEY_ID
```

This outputs the full `-----BEGIN PGP PRIVATE KEY BLOCK-----` ... `-----END PGP PRIVATE KEY BLOCK-----` text.

## 4. Add GitHub Repository Secrets

Go to: `https://github.com/rabindra-basnet/system-monitor/settings/secrets/actions`

Click **"New repository secret"** and add:

| Secret Name | Value |
|-------------|-------|
| `APT_GPG_KEY_ID` | The 16-char key ID from step 2 |
| `APT_GPG_PRIVATE_KEY` | Full output from step 3 (including BEGIN/END lines) |

## 5. How It Works

When you push a tag (`v*`) or trigger the release workflow manually:

1. `.deb` packages are built for amd64 and arm64
2. The workflow imports your GPG key from `APT_GPG_PRIVATE_KEY`
3. `Packages`, `Packages.gpg`, `InRelease`, and `Release.gpg` are generated and signed
4. Everything is committed to the `gh-pages` branch

## 6. User Installation

After publishing, users add the APT repository with:

```bash
# Import the signing key
wget -qO - https://rabindra-basnet.github.io/system-monitor/key.gpg | sudo gpg --dearmor -o /usr/share/keyrings/stasis.gpg

# Add the repository
echo "deb [signed-by=/usr/share/keyrings/stasis.gpg] https://rabindra-basnet.github.io/system-monitor stable main" | sudo tee /etc/apt/sources.list.d/stasis.list

# Install
sudo apt update && sudo apt install stasis
```

## Key Storage Location

GPG keys are stored in:

```
~/.gnupg/private-keys-v1.d/    # private key
~/.gnupg/openpgp-revocs.d/     # revocation certificate
```

Do **not** commit the private key to the repository. It is only stored as a GitHub Actions secret.
