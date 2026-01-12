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
- docker-compose.yml 配置
- .env ファイル生成（mode: 0644）
- cloudflared イメージプル
- サービス起動

**Docker イメージのデプロイ方式:**

テスト環境では、メモリ制約（1GB）のある VPS でのビルドを避けるため、ローカルでビルドして直接転送する：

```bash
# ローカルでビルド
docker build -f Dockerfile.signaling -t jamjam-signaling:latest .
docker build -f Dockerfile.echo -t jamjam-echo:latest .

# VPS に直接転送
docker save jamjam-signaling:latest | ssh user@vps docker load
docker save jamjam-echo:latest | ssh user@vps docker load
```

本番環境では ghcr.io からプルする方式も検討可能。

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

### テスト環境デプロイ（完全手順）

#### 1. 設定ファイルの準備

```bash
cd ansible

# inventory ファイルをコピー
cp inventory/test.yml.example inventory/test.yml

# group_vars ファイルをコピー
cp group_vars/test.yml.example group_vars/test.yml
```

#### 2. 設定ファイルの編集

**inventory/test.yml:**
```yaml
# ansible_host: VPS の IP アドレスを設定（PrivateDocs/secrets.md 参照）
ansible_host: <YOUR_VPS_IP>
```

**group_vars/test.yml:**
```yaml
# sudo パスワード
ansible_become_pass: "YOUR_SUDO_PASSWORD"

# Echo Server の公開アドレス
jamjam_echo_public_addr: "YOUR_SERVER_IP:5000"

# Cloudflare Tunnel トークン（Cloudflare ダッシュボードから取得）
jamjam_cloudflare_tunnel_token: "YOUR_TUNNEL_TOKEN"
```

#### 3. Docker イメージのビルドと転送

テスト VPS はメモリ 1GB のため、ローカルでビルドして転送する：

```bash
cd /path/to/p2paudiosession

# signaling-server イメージをビルド
docker build -f Dockerfile.signaling -t jamjam-signaling:latest .

# echo-server イメージをビルド
docker build -f Dockerfile.echo -t jamjam-echo:latest .

# VPS に転送（SSH 鍵のパスは環境に合わせて変更）
docker save jamjam-signaling:latest | ssh -i PrivateDocs/jamjam_vps ubuntu@<YOUR_VPS_IP> 'docker load'
docker save jamjam-echo:latest | ssh -i PrivateDocs/jamjam_vps ubuntu@<YOUR_VPS_IP> 'docker load'
```

#### 4. Ansible Playbook の実行

```bash
cd ansible
ansible-playbook -i inventory/test.yml playbooks/site.yml
```

#### 5. デプロイ確認

```bash
# SSH でサービス状態を確認
ssh -i PrivateDocs/jamjam_vps ubuntu@<YOUR_VPS_IP> 'cd /opt/jamjam && docker compose ps'

# 期待される出力:
# jamjam-signaling   ... healthy
# jamjam-echo        ... healthy
# jamjam-cloudflared ... running
```

### イメージ更新時の再デプロイ

コード変更後の再デプロイ手順：

```bash
# 1. イメージを再ビルド
docker build -f Dockerfile.signaling -t jamjam-signaling:latest .
docker build -f Dockerfile.echo -t jamjam-echo:latest .

# 2. VPS に転送
docker save jamjam-signaling:latest | ssh -i PrivateDocs/jamjam_vps ubuntu@<YOUR_VPS_IP> 'docker load'
docker save jamjam-echo:latest | ssh -i PrivateDocs/jamjam_vps ubuntu@<YOUR_VPS_IP> 'docker load'

# 3. サービス再起動
ssh -i PrivateDocs/jamjam_vps ubuntu@<YOUR_VPS_IP> 'cd /opt/jamjam && docker compose up -d'
```

### 本番環境デプロイ（Vault 使用）

本番環境では Ansible Vault で機密情報を暗号化する：

```bash
# 1. Vault パスワードファイル作成
echo "your-vault-password" > PrivateDocs/ansible-vault-password
chmod 600 PrivateDocs/ansible-vault-password

# 2. secrets.yml 作成・暗号化
ansible-vault create ansible/vault/secrets.yml --vault-password-file PrivateDocs/ansible-vault-password

# 3. 本番環境にデプロイ
ansible-playbook -i ansible/inventory/production.yml ansible/playbooks/site.yml \
  --vault-password-file PrivateDocs/ansible-vault-password
```

## inventory ファイル形式

inventory ファイルでは `children` 構造を使用する。これにより `group_vars/<group_name>.yml` が自動的に読み込まれる。

### inventory/test.yml

```yaml
all:
  children:
    test:
      hosts:
        test-jamjam:
          ansible_host: <TEST_VPS_IP>
          ansible_user: ubuntu
          ansible_ssh_private_key_file: "{{ inventory_dir }}/../../PrivateDocs/jamjam_vps"
          # ansible_become_pass is defined in group_vars/test.yml
```

### inventory/production.yml

```yaml
all:
  children:
    production:
      hosts:
        prod-jamjam:
          ansible_host: <PROD_VPS_IP>
          ansible_user: ubuntu
          ansible_ssh_private_key_file: "{{ inventory_dir }}/../../PrivateDocs/jamjam_vps"
          # ansible_become_pass is defined in group_vars/production.yml
```

**重要**: `children: <group_name>:` 構造を使用しないと、対応する `group_vars/<group_name>.yml` が読み込まれない。

## 依存関係

- Ansible >= 2.15
- Python >= 3.10
- 対象サーバー: Ubuntu 24.04 LTS

## セキュリティ考慮事項

### 機密情報の管理

機密情報を含むファイルは `.gitignore` で除外し、`.example` ファイルをテンプレートとして提供する：

```
# .gitignore
ansible/inventory/test.yml
ansible/inventory/production.yml
ansible/group_vars/test.yml
ansible/group_vars/production.yml
```

| ファイル | 内容 | Git 管理 |
|---------|------|---------|
| `inventory/test.yml` | サーバー IP | ❌ 除外 |
| `inventory/test.yml.example` | テンプレート | ✅ 管理 |
| `group_vars/test.yml` | パスワード、トークン | ❌ 除外 |
| `group_vars/test.yml.example` | テンプレート | ✅ 管理 |

### group_vars に含める機密情報

```yaml
# group_vars/test.yml
ansible_become_pass: "YOUR_SUDO_PASSWORD"
jamjam_cloudflare_tunnel_token: "YOUR_TUNNEL_TOKEN"
```

### Ansible Vault（オプション）

より厳格なセキュリティが必要な場合は Ansible Vault を使用：

- Vault パスワードファイルは Git 管理対象外
- 詳細は「Ansible Vault」セクションを参照

### その他

- SSH 鍵は PrivateDocs に保管
- ubuntu ユーザーで接続し、sudo で権限昇格
- Cloudflare Tunnel により、signaling-server は直接インターネットに公開しない
