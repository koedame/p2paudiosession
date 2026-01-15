---
name: test-server-deploy
description: Deploy services to test server via SSH. Builds Docker images locally and transfers them.
allowed-tools: Bash(docker *), Bash(ansible-playbook *), Bash(gzip *), Bash(cargo run *), Bash(ssh *), Bash(rm *), Bash(ls *), Bash(sleep *), Read
---

# Test Server Deploy

Deploy jamjam services (signaling-server, echo-server, cloudflared) to the test server.

## Instructions

### 1. Build Docker Images

Build both images locally:

```bash
docker build -f Dockerfile.signaling -t jamjam-signaling:latest .
docker build -f Dockerfile.echo -t jamjam-echo:latest .
```

### 2. Export Images

Save and compress the images for transfer:

```bash
docker save jamjam-signaling:latest jamjam-echo:latest | gzip > /tmp/jamjam-images.tar.gz
```

Report the file size to the user.

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
rm /tmp/jamjam-images.tar.gz
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

1. Check SSH connectivity:
   ```bash
   ssh -i PrivateDocs/jamjam_vps ubuntu@160.16.61.28 "echo OK"
   ```

2. Check Docker images on server:
   ```bash
   ssh -i PrivateDocs/jamjam_vps ubuntu@160.16.61.28 "docker images | grep jamjam"
   ```

3. Check container logs:
   ```bash
   ssh -i PrivateDocs/jamjam_vps ubuntu@160.16.61.28 "cd /opt/jamjam && docker compose logs --tail=50"
   ```
