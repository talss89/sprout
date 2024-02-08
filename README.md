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

## Tutorial: Using Sprout for the first time

**1. Set up a new Sprout repository**

When using Sprout for the first time, you'll need to set up a repository. This is a location that will store all of your data. You'll usually want this to be some network storage of some kind (think S3?), but for the purposes of this demo, we will use a local directory.

Let's create a repo definition:

`sprout repo new my-repo`

Sprout will fire up a text editor, and let you customise the repo definiton. You can leave access_key blank (it'll be generated for you if so), but **make sure to set `repository` to a new local directory**. How about `/tmp/my-repo`?

Save and quit the editor.

Next, we will initialise the repo. If you were connecting to an existing Sprout repo, you could skip this next step.

`sprout repo init my-repo`

Sprout will generate an access key for you - this is the encryption key for your data. If you lose it, you lose your data. It will be stored in the repo definition file though, so don't worry about having it to hand all of the time.

**2. Initialise your WordPress project**

To create a `sprout.yaml` file, which will identify which exact version of database and uploads content should be used when seeding, make sure you're in an existing WordPress project and simply run:

`sprout init`

Sprout will open another editor, and let you customise the project name and other settings. The defaults should be fine (and Sprout will detect if you've moved your wp-uploads folder, like Bedrock or Radicle does).

Close and save the editor. You can commit your new `sprout.yaml` to your git repo if you'd like.

**3. Take a snapshot**

Still inside the WordPress project, run

`sprout snap`

Sprout will then snapshot your database and entire wp-content folder and push it to the repo you just set up. It's that simple.

Your `sprout.yaml` file will have been updated with the latest snapshot ID. If you commit and push this to git, other developers can easily seed from the snapshot you just took.

**4. Seed the project**

Let's pointlessly seed the project we've just snapshotted. Just run:

`sprout seed`

Sprout will first stash all of your database and wp-uploads content locally (to avoid any lost data), and then will restore from the snapshot that's described by your `sprout.yaml`.

### What's going on behind the scenes?

Sprout stores your data in the Restic Repo Format, and uses rustic-rs/rustic-core internally. We dump the database via WP CLI, and replace the site URL with a placeholder value before storing it alongside an encrypted and de-duplicated archive of your wp-uploads folder.

Sprout rewrites the Restic hostname and path properties of each snapshot so your team can `snap` and `seed` seamlessly. Content branches are implemented as a virtual directory inside a snapshot.
