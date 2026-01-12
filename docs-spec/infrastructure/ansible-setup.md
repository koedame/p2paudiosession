# Ansible セットアップ仕様

## 概要

jamjam サーバーインフラを Ansible で構成管理する。
テスト環境と本番環境を同一の playbook で管理し、環境差分は変数で制御する。

## ディレクトリ構成

```
ansible/
├── ansible.cfg              # Ansible 設定
├── inventory/
│   ├── test.yml             # テスト環境ホスト
│   └── production.yml       # 本番環境ホスト
├── playbooks/
│   ├── site.yml             # メイン playbook
│   ├── deploy.yml           # デプロイのみ
│   └── rollback.yml         # ロールバック
├── roles/
│   ├── common/              # 共通設定
│   ├── docker/              # Docker インストール
│   ├── firewall/            # ファイアウォール設定
│   └── jamjam/              # jamjam サービス
├── group_vars/
│   ├── all.yml              # 全環境共通変数
│   ├── test.yml             # テスト環境変数
│   └── production.yml       # 本番環境変数
└── vault/
    └── secrets.yml          # 暗号化された秘匿情報
```

## ロール定義

### common ロール

基本的なサーバー設定を行う。

**タスク:**
- タイムゾーン設定（Asia/Tokyo）
- ロケール設定（en_US.UTF-8）
- 必要パッケージインストール（curl, git, htop, etc.）
- SSH 鍵設定
- swap 設定（メモリ 1GB 未満の VPS 向け）

### docker ロール

Docker と Docker Compose をインストールする。

**タスク:**
- Docker 公式リポジトリ追加
- Docker Engine インストール
- Docker Compose インストール
- jamjam ユーザーを docker グループに追加
- Docker デーモン設定（ログローテーション）

**変数:**
| 変数名 | デフォルト | 説明 |
|--------|-----------|------|
| `docker_log_max_size` | `100m` | ログファイル最大サイズ |
| `docker_log_max_file` | `5` | ログファイル最大数 |

### firewall ロール

ufw を使用してファイアウォールを設定する。

**タスク:**
- ufw インストール・有効化
- デフォルトポリシー設定（incoming: deny, outgoing: allow）
- 許可ルール追加

**変数:**
| 変数名 | デフォルト | 説明 |
|--------|-----------|------|
| `firewall_ssh_port` | `22` | SSH ポート |
| `firewall_allow_udp_5000` | `true` | Echo Server 用 UDP 5000 許可 |

### jamjam ロール

jamjam サービス（signaling-server, echo-server, cloudflared）をデプロイする。

**タスク:**
- アプリケーションディレクトリ作成（/opt/jamjam）
- Docker イメージビルド or プル
- docker-compose.yml 配置
- .env ファイル生成
- サービス起動

**変数:**
| 変数名 | デフォルト | 説明 |
|--------|-----------|------|
| `jamjam_app_dir` | `/opt/jamjam` | アプリケーションディレクトリ |
| `jamjam_rust_log` | `info` | ログレベル |
| `jamjam_echo_delay_ms` | `3000` | Echo 遅延（ms） |
| `jamjam_echo_public_addr` | `{{ ansible_default_ipv4.address }}:5000` | Echo 公開アドレス |
| `jamjam_signaling_url` | `ws://signaling-server:8080` | Signaling URL（内部） |
| `jamjam_cloudflare_tunnel_token` | (vault) | Cloudflare Tunnel トークン |

## 環境別変数

### group_vars/all.yml（共通）

```yaml
# ユーザー設定
jamjam_user: jamjam
jamjam_group: jamjam
jamjam_app_dir: /opt/jamjam

# Docker 設定
docker_log_max_size: 100m
docker_log_max_file: 5

# ネットワーク
jamjam_signaling_port: 8080
jamjam_echo_port: 5000

# ヘルスチェック
healthcheck_interval: 30s
healthcheck_timeout: 3s
healthcheck_retries: 3
```

### group_vars/test.yml（テスト環境）

```yaml
# 環境識別
environment_name: test

# ログ設定
jamjam_rust_log: debug

# Echo Server
jamjam_echo_delay_ms: 1000

# R2 ログアップロード
r2_log_upload_enabled: false
```

### group_vars/production.yml（本番環境）

```yaml
# 環境識別
environment_name: production

# ログ設定
jamjam_rust_log: info

# Echo Server
jamjam_echo_delay_ms: 3000

# R2 ログアップロード
r2_log_upload_enabled: true
r2_log_upload_interval: "0 * * * *"  # 毎時
```

## Ansible Vault

### 暗号化対象

以下の秘匿情報を `vault/secrets.yml` に保存し、暗号化する：

```yaml
# Cloudflare
vault_cloudflare_tunnel_token_test: "..."
vault_cloudflare_tunnel_token_production: "..."

# R2 Storage
vault_r2_access_key_id: "..."
vault_r2_secret_access_key: "..."
vault_r2_account_id: "..."

# SSH
vault_ssh_private_key: |
  -----BEGIN OPENSSH PRIVATE KEY-----
  ...
  -----END OPENSSH PRIVATE KEY-----
```

### Vault パスワード管理

Vault パスワードは `PrivateDocs/ansible-vault-password` に保存する。
このファイルは Git 管理対象外。

### Vault コマンド

```bash
# 暗号化
ansible-vault encrypt vault/secrets.yml

# 復号（編集）
ansible-vault edit vault/secrets.yml

# Playbook 実行時
ansible-playbook -i inventory/test.yml playbooks/site.yml --vault-password-file PrivateDocs/ansible-vault-password
```

## 実行手順

### 初回セットアップ

```bash
# 1. Vault パスワードファイル作成
echo "your-vault-password" > PrivateDocs/ansible-vault-password
chmod 600 PrivateDocs/ansible-vault-password

# 2. secrets.yml 作成・暗号化
ansible-vault create ansible/vault/secrets.yml --vault-password-file PrivateDocs/ansible-vault-password

# 3. テスト環境にデプロイ
ansible-playbook -i ansible/inventory/test.yml ansible/playbooks/site.yml \
  --vault-password-file PrivateDocs/ansible-vault-password

# 4. 本番環境にデプロイ
ansible-playbook -i ansible/inventory/production.yml ansible/playbooks/site.yml \
  --vault-password-file PrivateDocs/ansible-vault-password
```

### デプロイのみ

```bash
ansible-playbook -i ansible/inventory/test.yml ansible/playbooks/deploy.yml \
  --vault-password-file PrivateDocs/ansible-vault-password
```

### ロールバック

```bash
ansible-playbook -i ansible/inventory/test.yml ansible/playbooks/rollback.yml \
  --vault-password-file PrivateDocs/ansible-vault-password
```

## inventory ファイル形式

### inventory/test.yml

```yaml
all:
  hosts:
    test-vps:
      ansible_host: <TEST_VPS_IP>
      ansible_user: root
      ansible_ssh_private_key_file: PrivateDocs/jamjam_vps
  vars:
    cloudflare_tunnel_token: "{{ vault_cloudflare_tunnel_token_test }}"
```

### inventory/production.yml

```yaml
all:
  hosts:
    prod-vps:
      ansible_host: <PROD_VPS_IP>
      ansible_user: root
      ansible_ssh_private_key_file: PrivateDocs/jamjam_vps
  vars:
    cloudflare_tunnel_token: "{{ vault_cloudflare_tunnel_token_production }}"
```

## 依存関係

- Ansible >= 2.15
- Python >= 3.10
- 対象サーバー: Ubuntu 24.04 LTS

## セキュリティ考慮事項

- Vault パスワードファイルは Git 管理対象外
- SSH 鍵は PrivateDocs に保管
- 本番環境への直接 root ログインは初回のみ、以降は jamjam ユーザーを使用
- Cloudflare Tunnel により、signaling-server は直接インターネットに公開しない
