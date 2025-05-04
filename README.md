# FerrOS

シンプルな UEFI ベースのオペレーティングシステム。フレームバッファを直接操作して画面に表示を行います。

## 機能

- UEFI ブートローダーからの起動
- グラフィックスアウトプットプロトコル（GOP）を使用した画面表示
- カスタムフォントによるテキスト描画

## プロジェクト構造

```
ferr_os/
├── src/                  # ソースコード
│   └── main.rs           # メインエントリポイント
├── .cargo/               # Cargoの設定
│   └── config.toml       # ビルド設定
├── scripts/              # 実行スクリプト
│   └── launch_qemu.sh    # QEMU起動スクリプト
├── mnt/                  # ビルド時に自動生成されるマウントポイント
│   └── EFI/              # EFIシステムパーティション
│       └── BOOT/         # ブートローダーディレクトリ
│           └── BOOTX64.EFI  # 実行可能なEFIファイル
└── third_party/          # サードパーティのコンポーネント
    └── ovmf/             # OVMFファイル（UEFI BIOS）
        ├── OVMF_CODE.fd  # UEFI BIOSコード
        ├── OVMF_VARS.fd  # UEFI BIOS変数
        └── RELEASEX64_OVMF.fd # 64bit UEFI BIOSイメージ
```

## 環境構築

### 必要なもの

- Rust (nightly)
- QEMU
- UEFI ターゲット用のツールチェーン

### OVMF ファイルの入手

OVMF ファイル（UEFI BIOS）は以下の手順で入手し、適切に配置してください：

1. EDK2 プロジェクトからビルド済みの OVMF ファイルをダウンロード：

   - [EDK2 OVMF Releases](https://github.com/tianocore/edk2/releases)
   - または、OS によってはパッケージマネージャーからインストール可能：
     - Ubuntu: `apt install ovmf`
     - Arch Linux: `pacman -S edk2-ovmf`
     - macOS: `brew install qemu` (OVMF ファイルが含まれています)

2. 以下のファイルを `third_party/ovmf/` ディレクトリに配置：
   - OVMF_CODE.fd
   - OVMF_VARS.fd
   - RELEASEX64_OVMF.fd (オプション)

### ビルド方法

ビルドと実行は以下のコマンドで行えます：

```
cargo run
```

または手動でビルドする場合：

```
cargo build --target x86_64-unknown-uefi
```

## 開発状況

現在は基本的なフレームバッファ操作とテキスト表示機能を実装しています。これから以下の機能を追加予定です：

- メモリ管理
- プロセス管理
- ファイルシステム

## ライセンス

MIT ライセンス

## ドキュメント

詳細なロードマップや設計資料は `docs/` ディレクトリ（mdBook 形式）にまとめています。ブラウザで閲覧する場合は以下を実行してください：

```bash
cargo install mdbook # 初回のみ
mdbook serve docs
```

ローカルサーバーが立ち上がり、`http://localhost:3000` で閲覧できます。
