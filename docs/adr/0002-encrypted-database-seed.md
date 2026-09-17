# Keep the imported database seed encrypted

The uploaded PostgreSQL backup is committed only as AES-256 encrypted
ciphertext; its passphrase is a GitHub Actions secret and is copied into the
project runtime secret during deployment. This makes the initial data restore
reproducible without placing permit, calendar, trip, or personal data in Git
plaintext. Runtime backups are encrypted with the same project key before they
enter the private backup bucket.
