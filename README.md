# Vendor Space

Need to build your rust code offline? Vendor it! vendor-space was created to facilitate local offline development of multiple rust projects.

A vendor space is a folder containing vendored rust repositories. The space is configured with vendor-space.toml, where you define the repositories that you would like to manage. The `vendor-space` tool automates the process of acquiring and vendoring the code for any branches you specify.

## vendor-space.toml

Before running `vendor-space`, you must create a configuration file.

```toml
# Optional. Defines the root directory of the vendor space.
# Defaults to the directory containing the config file, or "."
root = "."

# Optional. Default = false
# Sets the default value of allow_existing that will be used for
# repositories that do not set an explicit value here.
allow_existing = true

# Each section defines a repository to vendor. Choose any name here,
# and it will be the name of the folder containing the repository.
[clipboard-sync]

# Optional. Default = false
# If false, vendor-space expects that it needs to clone the repository from scratch. 
# If the repository already exists, vendor-space will fail with an error.
# true means it will just use the existing folder, if it exists and is a valid git 
# repo, and otherwise clone it.
allow_existing = true

# Required.
# The git URL used to clone the repository if necessary.
url = "git@github.com:dnut/clipboard-sync.git"

# Optional. Default = ["master"]
# The branches you would like to vendor. Each gets a `.cargo/<branch>.config.toml` in the repository.
# The first branch in the list will be used for:
# - `.cargo/config.toml`
# - the currently checked out branch
branches = ["master", "release"]
```

## Run it

After you have a configuration file, you can initialize the vendor space simply with the `vendor-space` command:

```bash
$ vendor-space
```

You can customize the behavior of the program, for example with alternative locations for the config file or vendor space root, with some additional command line options that are described in the help document:

```bash
$ vendor-space --help
```
