# Documentation

This folder is the long-form reference for `stellar-forge`.

## Suggested Reading Order

1. [README](../README.md) for the project overview and quick start
2. [Command reference](command-reference.md) for the exact CLI surface
3. [Manifest and state reference](manifest-reference.md) for `stellarforge.toml`,
   `stellarforge.lock.json`, generated files, and environment variables
4. [Deployment guide](deployment-guide.md) for local, testnet, futurenet, pubnet, and registry
   rollout workflows

## What Each Document Covers

| Document | Best used for |
| --- | --- |
| [README](../README.md) | First install, quick orientation, common workflows |
| [command-reference.md](command-reference.md) | Looking up syntax, flags, and examples |
| [manifest-reference.md](manifest-reference.md) | Understanding how the manifest, lockfile, references, and generated outputs fit together |
| [deployment-guide.md](deployment-guide.md) | Planning or executing a deploy to local/testnet/pubnet, and understanding deploy artifacts |

## Fast Answers

### I just want to bootstrap a project

Start with:

```bash
stellar forge init hello-stellar --template fullstack --network testnet
cd hello-stellar
stellar forge doctor
stellar forge project validate
```

### I want the full command surface

Go to [command-reference.md](command-reference.md).

### I want to understand the config model

Go to [manifest-reference.md](manifest-reference.md).

### I need a deploy checklist

Go to [deployment-guide.md](deployment-guide.md).
