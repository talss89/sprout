# Sprout 
*Fast, deduplicated content and database seeding for WordPress.*

**[Documentation](https://talss89.github.io/sprout/) | [Install](https://talss89.github.io/sprout/install/) | [Releases](https://github.com/talss89/sprout/releases)**

- Store your uploads and database in a secure, central location and easily let your team get up and running with the correct version of content **with a one-liner - `sprout seed`**.
  
- Sprout lets you commit a small `sprout.yaml` file as part of your WordPress project. It identifies the **exact version of database and wp-uploads content** required in order to seed a new environment.

- You can store your database and content in a Sprout repo - this can be a local path, SFTP, S3 bucket, or any number of supported backends via OpenDAL or rclone. **Have complete control over your data.**

- Your data is **encrypted and deduplicated**, and stored in the proven and trusted Restic format. So much more efficient than chucking ZIPs around.

- **Sprout is FAST**. Written in Rust, and provided as a single static binary, it will run anywhere.

- Sprout will **locally stash your current database and wp-uploads during destructive operations** - a nice safety net to have!

---

:warning: **Although I think you'll love it, Sprout is pre-release. You should always back up your WordPress content and database before using Sprout.**

---

[![asciicast](https://asciinema.org/a/641208.svg)](https://asciinema.org/a/641208)

## Installation

Binaries are avalailable for Mac and Linux on both x86_64 and arm64. You can get them via the [releases page](https://github.com/talss89/sprout/releases).

### Mac

```bash
# x86_64 / Intel Macs
curl -L "https://github.com/talss89/sprout/releases/latest/download/sprout-macos-x86_64.tar.gz" | tar zxf - && sudo install -c -m 0755 sprout /usr/local/bin

# aarch64 / M1 Macs
curl -L "https://github.com/talss89/sprout/releases/latest/download/sprout-macos-aarch64.tar.gz" | tar zxf - && sudo install -c -m 0755 sprout /usr/local/bin
```

### Linux

```bash
# x86_64
curl -L "https://github.com/talss89/sprout/releases/latest/download/sprout-linux-x86_64.tar.gz" | tar zxf - && sudo install -c -m 0755 sprout /usr/local/bin

# aarch64 / amd64
curl -L "https://github.com/talss89/sprout/releases/latest/download/sprout-linux-aarch64.tar.gz" | tar zxf - && sudo install -c -m 0755 sprout /usr/local/bin
```

Windows support is untested, and will require compilation with `cargo`.

### What's going on behind the scenes?

Sprout stores your data in the Restic Repo Format, and uses [rustic-rs/rustic_core](https://github.com/rustic-rs/rustic_core) internally. We dump the database via WP CLI, and replace the site URL with a placeholder value before storing it alongside an encrypted and de-duplicated archive of your wp-uploads folder.

Sprout rewrites the Restic hostname and path properties of each snapshot so your team can `snap` and `seed` seamlessly. Content branches are implemented as a virtual directory inside a snapshot.
