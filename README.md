# rclone-sync

Tiny CLI for pulling selected PDFs from an `rclone` remote to fixed local paths.

I use it with the OS task scheduler to periodically pull Noteshelf backups down to my machine.

The cloud side is the source of truth: it lists files in `remote:<folder>`, keeps `.pdf` files, and copies the ones in `[map]` to their local destinations.

Open the config with:

```bash
rclone-sync config
```

Run a sync with:

```bash
rclone-sync sync
```

If the config does not exist yet, it is created from `.rclone-sync.example.toml`. `rclone` must be on your `PATH`.
