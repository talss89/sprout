# Sprout 
Fast, deduplicated content and database seeding for WordPress.

- Store your uploads and database in a secure, central location and easily let your team get up and running with the correct version of content **with a one-liner - `sprout seed`**.
  
- Sprout lets you commit a small `sprout.yaml` file as part of your WordPress project. It identifies the **exact version of database and wp-uploads content** required in order to seed a new environment.

- You can store your database and content in a Sprout repo - this can be a local path, SFTP, S3 bucket, or any number of supported backends via OpenDAL or rclone. **Have complete control over your data.**

- Your data is **encrypted and deduplicated**, and stored in the proven and trusted Restic format. So much more efficient than chucking ZIPs around.

- **Sprout is FAST**. Written in Rust, and provided as a single static binary, it will run anywhere.

- Sprout will **locally stash your current database and wp-uploads during destructive operations** - a nice safety net to have!

---

:warning: **This is a very early preview release, and should be used only when you've backed up your databases and uploads. I'm not responsible for anything bad that happens.**

---

[![asciicast](https://asciinema.org/a/636443.svg)](https://asciinema.org/a/636443)

## Installation

Binaries are avalailable for macos and linux on both x86_64 and arm64. You can get them via the [releases page](https://github.com/talss89/sprout/releases). Untar and put in your `PATH`.

An installer will come soon.

Windows support is untested, and will require compilation with `cargo`.

I wrote Sprout after years of being sent SQL files and gigabytes of TAR archives when working as a consultant on WordPress projects. There had to be a better way, and I think this is it.

**This is a very early preview release, and should be used only when you've backed up your databases and uploads. I'm not responsible for anything bad that happens.**


## How it works

First, you'll need to set up a repository. This is essentially just a directory that will store your snapshots. `sprout repo init <repo-path>` will get you started. Set a secure access key, and make sure you keep it safe.

Then, simply change into your WordPress project directory, and run `sprout init`. Customise the generated `sprout.yaml` and set the `repo.repository` to the path or URI of your repository you set up previously.

Once set up, you can snapshot your uploads and database. Run `sprout snap`, and Sprout will snapshot your uploads and database and push them to your repository. It will also update your `sprout.yaml` file to point to this latest snapshot. This means, when you commit the `sprout.yaml` and share with your team, they will be able to easily run `sprout seed` to easily pull down your snapshot and get up and running in no time.

### What's going on behind the scenes?

Sprout stores your data in the Restic Repo Format, and uses rustic-rs/rustic-core internally. We dump the database via WP CLI, and replace the site URL with a placeholder value before storing it alongside an encrypted and de-duplicated archive of your wp-uploads folder.

Sprout rewrites the Restic hostname and path properties of each snapshot so your team can `snap` and `seed` seamlessly. Content branches are implemented as a virtual directory inside a snapshot.
