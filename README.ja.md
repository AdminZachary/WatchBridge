<div align="center">

<img src="public/watchbridge.svg" width="104" alt="WatchBridge" />

# WatchBridge

**Windows で Apple Watch の開発者モードを準備し、Watch Companion アプリを Watch にインストールします。**

[English](README.en.md) · [简体中文](README.md) · [한국어](README.ko.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [Español](README.es.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Português](README.pt.md)

[![Build Windows](https://img.shields.io/github/actions/workflow/status/AdminZachary/iloader-for-windows-/windows.yml?branch=main&label=windows&logo=github)](https://github.com/AdminZachary/iloader-for-windows-/actions/workflows/windows.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-0078D4)
![Status](https://img.shields.io/badge/hardware%20validated-2026--11--12-brightgreen)
![Version](https://img.shields.io/badge/version-0.1.0--alpha.1-lightgrey)

</div>

> [!IMPORTANT]
> WatchBridge は [AndreySemjonov の iLoader fork](https://github.com/AndreySemjonov/iloader) から派生した独立コミュニティ版です。上流は [nab138/iLoader](https://github.com/nab138/iloader) です。
> Apple や上流プロジェクトの**公式版ではなく、いかなる承認も意味しません**。問題は本リポジトリへ報告してください。

> [!NOTE]
> iPhone 17 Pro + Apple Watch Ultra 2、iPhone 13 Pro + Apple Watch Series 7 の 2 組で全工程を確認済みです:
> 開発者モードの準備 → 署名 → iPhone へのインストール → Watch へのインストール → アプリ起動。

## これは何か

Windows で自己署名の iOS アプリ（SideStore など）を入れるには 3 つが必要です。Apple ID による署名、対象端末の開発用端末としての登録、そして端末の開発者モード有効化です。公式のツールチェーンは Xcode で、これは macOS 専用です。

iLoader はその半分（iPhone 側）を Rust で Windows に移植しました。**WatchBridge が補うのは残りの半分**です。iPhone に入れたアプリ内の `Watch/*.app` をペアリング済みの Apple Watch に届け、その前に Watch の開発者モードスイッチを表示させます。Python のインストールもコマンドラインも不要です。

端末との通信は回り道をします。Windows からは Watch が見えないため、プログラムは iPhone をルーター代わりに使い、ポートを Watch へ転送し、その転送セッションの中でペアリング・状態取得・インストールを行います。

## 機能

| | 機能 | 内容 |
|---|---|---|
| 🛠 | **開発者モードの準備** | Watch をペアリング → スイッチの表示を要求 → 状態を読み戻す。送るのは `action 0` のみで、**有効化・再起動・確認は常に Watch 上で利用者が行います** |
| 📦 | **Companion のインストール** | IPA に iPhone 側のホストと `Watch/*.app` の両方が含まれるか検証し、再署名して iPhone に入れたあと Watch アプリをインストールします |
| 📶 | **Wi-Fi / RSD** | USB で一度 Wi-Fi を設定すれば、以降は無線でインストールと更新ができます。開発者モードの準備には USB が必要です |
| ♻️ | **自動更新** | 7 日の署名期限前に自動で再署名。トレイ操作と任意のスタートアップ登録に対応 |
| 🔐 | **アカウントと証明書** | Apple ID ログイン（anisette が端末 ID を提供）、証明書の確認・失効、App ID 管理、keyring 対応 |
| 🔗 | **ペアリングファイル** | pairing file の入出力、Wi-Fi ペアリングの修復 |
| 🌍 | **多言語 UI** | 28 言語（簡体字・繁体字中国語を含む） |

## 使い方の流れ

```text
① ペアリング済み iPhone を USB 接続   ② Watch をペアリングし、Watch で信頼を確認
   端末を選択                             「開発者モード」スイッチの表示を要求
   ↓                                      ↓
   Watch で手動で有効化 → 再起動 → 確認 → 「再確認」をクリック
   ↓
③ Apple ID でログイン → 元の IPA を選択 → 署名してインストール
   ↓
   Watch で手動でアプリを開き、起動することを確認
```

> [!NOTE]
> 本プログラムは開発者モードを自動で有効化せず、Watch を再起動せず、信頼の確認も代行しません。信頼の記録はメモリ上にのみ存在し、プログラムを再起動すると新しいセッションが必要になります。

## 動作要件

- Windows 10 22H2 / Windows 11（x86_64）
- **Apple Mobile Device Service**: [iTunes](https://www.apple.com/itunes/) または Microsoft Store の Apple Devices アプリに含まれます。これがないと USB も Wi-Fi も動作しません
- WebView2 ランタイム（Windows 11 は標準搭載）
- 対象の Watch とペアリング済みの iPhone。準備段階ではケーブルが必要です

## インストールと実行

[Actions](https://github.com/AdminZachary/iloader-for-windows-/actions/workflows/windows.yml) の `windows` ジョブから `watchbridge-windows` アーティファクトを取得し、展開して `WatchBridge.exe` を実行します。

アーカイブの内容:

| ファイル | 内容 |
|---|---|
| `WatchBridge.exe` | 本体。**未署名**のため初回起動時に SmartScreen の警告が出ます |
| `BUILD.json` | ソースのコミット、作業ツリーの汚れ、ロックファイルのハッシュ、ツールチェーンのバージョン、exe の SHA-256 |
| `SHA256SUMS.txt` | 上記ファイルのチェックサム |
| `LICENSES.zip` | サードパーティのライセンス原文と依存一覧（`inventory.json`） |

## ソースからビルド

依存関係と手順は [BUILDING.md](BUILDING.md) を参照してください。GitHub Actions がビルドし、`iloader` と `isideload` の Rust ユニットテストを実行して、上記のアーティファクトを生成します。

```powershell
# フロントエンドのみ
bun install --frozen-lockfile --ignore-scripts
bun run build

# Windows のフルビルド（テスト + exe + チェックサム）
.\scripts\Build-Windows.ps1 -BuildExecutable
```

ローカルで速くビルドしたい場合は `-SkipTests` を付けてテストを省略できます。ビルドは Windows + MSVC のネイティブのみで、クロスコンパイルには対応しません。

## 互換性と既知の制限

| 制限 | 内容 |
|---|---|
| **Watch は 1 台** | iPhone に複数の Watch がペアリングされている場合、準備処理は推測せず拒否します。転送プロトコルに「対象 Watch の指定」がなく、誤った Watch を操作するほうが失敗より有害です |
| **Watch 単体パッケージ非対応** | iPhone 側のホストと `Watch/*.app` の両方を含む元の IPA が必要です |
| **App Group** | Watch App Group を必要とするアプリは、上流が関連付けを省略する挙動の影響を受ける可能性があります |
| **OS バージョン** | Watch 側の処理は xtool の実験的実装に由来し、iOS/watchOS の組み合わせごとに実機検証が必要です |
| **非公式ツール** | 脱獄なし、Apple の署名の回避なし、証明書の偽造なし |

## 検証状況

証拠の強さを混同しないよう、上流コミュニティの区分に従います:

| 区分 | 意味 | 現在 |
|---|---|---|
| `BUILD_VALIDATED` | ビルドでき、ユニットテストが通る | ✅ Windows CI |
| `CI_VALIDATED` | CI がダウンロード可能なアーティファクトを生成 | ✅ |
| `IPHONE_INSTALLED` | iPhone にインストールされ起動する | ✅ 実機 |
| `WATCH_INSTALLED` | Watch にアプリがインストールされる | ✅ 実機 |
| `PHYSICALLY_VALIDATED` | 全経路を実機で確認 | ✅ 2026-11-12 |

実機の記録:

| 日付 | 工程 | 端末 | 結果 |
|---|---|---|---|
| 2026-11-12 | 全工程: 準備 → 署名 → iPhone インストール → Watch インストール → 起動 | iPhone 17 Pro + Ultra 2 | ✅ 成功 |
| 2026-11-12 | 全工程 | iPhone 13 Pro + Series 7 | ✅ 成功 |

実機チェックリストは [docs/WATCHBRIDGE_TESTING.md](docs/WATCHBRIDGE_TESTING.md) にあります。

## よくある質問

<details>
<summary><b><code>fail to connect to usbmuxd device socket: io error</code></b></summary>

Windows の usbmuxd は Apple Mobile Device Service が提供し、`127.0.0.1:27015` で待ち受けます。このエラーはそのポートに応答がないことを意味し、多くはサービスが未インストールか停止しています。

```powershell
Get-Service "Apple Mobile Device Service"      # 存在し Running であること
Test-NetConnection 127.0.0.1 -Port 27015       # True であること
$env:USBMUXD_SOCKET_ADDRESS                    # 空であること
```

サービスがない場合は iTunes か Apple Devices アプリをインストールしてください。アドレスが上書きされている場合はその環境変数を削除します。本プログラムは usbmuxd を**同梱せず**、Apple のドライバとサービスに完全に依存します。
</details>

<details>
<summary><b>「Watch の情報を読み取れません」</b></summary>

以前のバージョンはペアリング前に iPhone のペアリングレジストリへ Watch 名を問い合わせており、新しい iOS ではこの問い合わせが値を返しません。現在はまず Watch と信頼関係を結び、Watch 自身のセッションから識別情報を読み取ります。それでも失敗する場合は「設定とログ → ログを表示」で `watch step failed` の `kind=` を確認してください。`timed out` は Watch が応答していない状態、`NotFound` / `UnexpectedResponse` はプロトコルの差異です。
</details>

<details>
<summary><b>「ペアリング済みの Watch が複数あります」</b></summary>

上記の「Watch は 1 台」の制限を参照してください。iPhone の Watch アプリで対象の Watch を確認してください。もう一方の Watch の電源を切ったり遠ざけたりしても、ペアリングレジストリからは**消えません**。
</details>

## コントリビューション

- 貢献ルールは [CONTRIBUTING.md](CONTRIBUTING.md) を参照してください
- 上流は AI 生成の貢献を明確に禁止しているため、**本リポジトリは上流へ PR を送りません**。変更はこの fork に留めてください
- Apple ID、パスワード、証明書の秘密鍵、ペアリング記録、マスクされていない端末ログを添付しないでください

## ライセンスと謝辞

本プロジェクトのコードは MIT で公開されています（[LICENSE](LICENSE)）。ブランドの利用条件は [LICENSE-BRANDING](LICENSE-BRANDING) にあります。

> 依存関係にある署名ライブラリ（`apple-crates` の `apple-codesign-quick`）はライセンスを宣言しておらず、条件は確認中です。詳細は [docs/LICENSE_REVIEW.md](docs/LICENSE_REVIEW.md) を参照してください。

出典と帰属: [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)、[vendor/isideload/PROVENANCE.md](vendor/isideload/PROVENANCE.md)。

| 出典 | 貢献 |
|---|---|
| [nab138/iLoader](https://github.com/nab138/iloader) | 元のアプリ、署名とインストールの処理 |
| [AndreySemjonov/iloader](https://github.com/AndreySemjonov/iloader) | 本リポジトリの起点: Wi-Fi/RSD と自動更新 |
| [oacevedo/xtool](https://github.com/oacevedo/xtool/tree/feat/experimental-watchos) | Watch の開発者モード準備の流れ（本リポジトリでは Rust でネイティブに再実装、Python なし） |
| Data_C0re_ (Rzbck) | Watch Companion の署名・登録・インストール |
| idevice / isideload と上流の作者 | 端末通信と署名の実装 |

## 免責事項

本プロジェクトは Apple Inc. と一切関係ありません。Apple、iPhone、Apple Watch は Apple Inc. の商標です。

学習・研究・個人開発の用途に限ります。利用者は居住地域の法令と、インストールするアプリのライセンス条件を自ら確認してください。端末の破損、データの損失、アカウントの制限、証明書の失効について作者は責任を負いません。
