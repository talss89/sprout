# sprout
Fast, deduplicated content and database seeding for WordPress

Sprout allows you and your team to easily snapshot or pull down entire archives of wp-uploads and database content.

- Commit a sproutfile to your project repo and let your team easily bootstrap new environments.
- Sprout supports multiple content channels. Working on a feature branch that relies on new content? Create a new channel.
- Store your content anywhere; locally, or on the cloud. Use any rclone provider.
- Sprout deduplicates your data - snapshotting is quick and lightweight.
- Sprout is a statically linked binary, written in Rust. It runs anywhere (your CI pipeline will eat it right up).
- Sprout can locally stash your database and content, meaning you can play more and worry less.

## How it works

Sprout stores your data in the Restic Repo Format, and uses rustic-rs/rustic-core internally. 

We bend the rules slightly to make this all work for WordPress...

Firstly, the snapshot hostname is now a project identifier (this uniquely identifies the WordPress project, and is derived from the git head SHA). Snapshot paths are also rewritten to support multiple content channels and database dumps.

## Restic mappings

Hostname: Project identifier - root SHA of repo?
Path: Branch and db / content ident


