# sprout

Fast, deduplicated content and database seeding for WordPress

_Sprout allows you and your team to easily snapshot or pull down entire archives of wp-uploads and database content, in a secure and efficient manner. Easily track seeded content in git, but store your gigabytes of uploads anywhere else._

- Sprout is a statically linked binary, written in Rust. It runs anywhere (your CI pipeline will eat it right up).
- Sprout deduplicates and encrypts your data - snapshotting is quick and lightweight.
- Sprout locally stashes your database and content when seeding, meaning you can easily revert.
- Sprout supports multiple content branches. Working on a feature that relies on new content? Create a new content branch.
- Store your content anywhere; locally, or on the cloud. Use S3, SFTP, HTTP or make use of tons of OpenDAL or Rclone providers. Even a local path is fine.
- Commit a `sprout.yaml` to your project repo and let your team easily bootstrap content in new environments.

I wrote Sprout after years of being sent SQL files and gigabytes of TAR archives when working as a consultant on WordPress projects. There had to be a better way, and I think this is it.

**This is a very early preview release, and should be used only when you've backed up your databases and uploads. I'm not responsible for anything bad that happens.**

## How it works

First, you'll need to set up a repository. This is essentially just a directory that will store your snapshots. `sprout repo init <repo-path>` will get you started. Set a secure access key, and make sure you keep it safe.

Then, simply change into your WordPress project directory, and run `sprout init`. Customise the generated `sprout.yaml` and set the `repo.repository` to the path or URI of your repository you set up previously.

Once set up, you can snapshot your uploads and database. Run `sprout snap`, and Sprout will snapshot your uploads and database and push them to your repository. It will also update your `sprout.yaml` file to point to this latest snapshot. This means, when you commit the `sprout.yaml` and share with your team, they will be able to easily run `sprout seed` to easily pull down your snapshot and get up and running in no time.

### What's going on behind the scenes?

Sprout stores your data in the Restic Repo Format, and uses rustic-rs/rustic-core internally. We dump the database via WP CLI, and replace the site URL with a placeholder value before storing it alongside an encrypted and de-duplicated archive of your wp-uploads folder.

Sprout rewrites the Restic hostname and path properties of each snapshot so your team can `snap` and `seed` seamlessly. Content branches are implemented as a virtual directory inside a snapshot.