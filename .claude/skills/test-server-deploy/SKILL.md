---
name: test-server-deploy
description: Deploy services to test server via SSH. Builds Docker images locally and transfers them.
allowed-tools: Bash(docker *), Bash(ansible-playbook *), Bash(gzip *), Bash(cargo run *), Bash(ssh *), Bash(rm *), Bash(ls *), Bash(sleep *), Read
---

# Test Server Deploy

Deploy jamjam services (signaling-server, echo-server, cloudflared) to the test server.

## Instructions

### 1. Build Docker Images

Build both images locally for AMD64 platform (test server is x86_64):

```bash
docker build --platform linux/amd64 -f Dockerfile.signaling -t jamjam-signaling:latest .
docker build --platform linux/amd64 -f Dockerfile.echo -t jamjam-echo:latest .
```

**Note**: The `--platform linux/amd64` flag is required when building on Apple Silicon (ARM64) Mac for deployment to x86_64 servers. If you encounter platform mismatch errors on the server, rebuild with `--no-cache`:

```bash
docker build --platform linux/amd64 --no-cache -f Dockerfile.signaling -t jamjam-signaling:latest .
docker build --platform linux/amd64 --no-cache -f Dockerfile.echo -t jamjam-echo:latest .
```

### 2. Export Images

Save and compress the images for transfer:

```bash
rm -f /tmp/jamjam-images.tar.gz
docker save jamjam-signaling:latest jamjam-echo:latest | gzip > /tmp/jamjam-images.tar.gz
ls -lh /tmp/jamjam-images.tar.gz
```

Report the file size to the user (typically ~35MB).

### 3. Deploy via Ansible

Run the Ansible playbook to deploy:

```bash
cd ansible && ansible-playbook -i inventory/test.yml playbooks/site.yml
```

This will:
- Transfer the Docker images to the server
- Load the images into Docker
- Create/update `.env` file with environment variables
- Create/update `docker-compose.yml`
- Start or restart services
- Wait for services to become healthy

### 4. Cleanup

Remove the temporary image archive:

```bash
rm -f /tmp/jamjam-images.tar.gz
```

### 5. Verify Server Status

Check service status on the server:

```bash
ssh -i PrivateDocs/jamjam_vps ubuntu@160.16.61.28 "cd /opt/jamjam && docker compose ps"
```

All three services should be running:
- `jamjam-signaling` - healthy
- `jamjam-echo` - healthy
- `jamjam-cloudflared` - running (no healthcheck)

### 6. Test CLI Connection

Test the signaling server connection using the CLI:

```bash
cargo run --release --bin jamjam -- rooms --server wss://test-signaling-jamjam.koeda.me
```

Expected output:
```
INFO Connecting to signaling server: wss://test-signaling-jamjam.koeda.me
INFO Connected, listing rooms...
Available rooms:
  xxxxxxxx - [BOT] Echo Server (1000ms delay) (1/10 peers)
```

If the Echo Server room appears, the deployment is successful.

## Server Information

| Item | Value |
|------|-------|
| Host | 160.16.61.28 |
| User | ubuntu |
| SSH Key | PrivateDocs/jamjam_vps |
| App Directory | /opt/jamjam |

## Services Deployed

- **signaling-server** - WebSocket signaling server (port 8080, localhost only)
- **echo-server** - Audio echo test server (port 5000/UDP)
- **cloudflared** - Cloudflare Tunnel for TLS termination

## Troubleshooting

If deployment fails:

1. **Platform mismatch error** ("The requested image's platform (linux/arm64) does not match the detected host platform (linux/amd64)"):
   - This occurs when building on Apple Silicon without specifying the target platform
   - Solution: Rebuild images with `--platform linux/amd64 --no-cache` flags
   - See Step 1 for the correct build commands

2. Check SSH connectivity:
   ```bash
   ssh -i PrivateDocs/jamjam_vps ubuntu@160.16.61.28 "echo OK"
   ```

3. Check Docker images on server:
   ```bash
   ssh -i PrivateDocs/jamjam_vps ubuntu@160.16.61.28 "docker images | grep jamjam"
   ```

4. Check container logs:
   ```bash
   ssh -i PrivateDocs/jamjam_vps ubuntu@160.16.61.28 "cd /opt/jamjam && docker compose logs --tail=50"
   ```
