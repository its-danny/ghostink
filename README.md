# 👻 ghostink

A CLI-driven, encrypted pastebin.

## How it works

1. **Create**: Your content is encrypted locally with a random key
2. **Upload**: Only the encrypted content goes to the server
3. **Share**: You get a command with the UUID and decryption key
4. **Retrieve**: Others use the command to fetch and decrypt

The server never sees your plaintext or encryption keys.

## Usage

### Creating a paste

```bash
# From stdin
echo "secret message" | ghostink create -

# From a file
ghostink create file.txt

# With expiration time
echo "temporary secret" | ghostink create - --expires 1h
```

Output:
```bash
ghostink get abc123#a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd
```

### Retrieving a paste

Use the exact command from the create output:

```bash
ghostink get abc123#a1b2c3d4e5f6789012345678901234567890123456789012345678901234abcd
```

### Expiration times

- `30s` - 30 seconds
- `5m` - 5 minutes
- `2h` - 2 hours
- `1d` - 1 day (default)
- `1w` - 1 week
