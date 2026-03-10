# ZeroMapper

[한국어](README.md) | [English](README.en.md) | **日本語**

8BitDo ZeroやMicroのような小型コントローラーを、**お絵描き用ショートカットパッド**に変えるWindows常駐アプリです。

アクティブなウィンドウを自動認識し、アプリごとにボタンマッピングを切り替えます。クリップスタジオではブラシ/消しゴム/元に戻す、Photoshopに切り替えると自動でPhotoshop用のショートカットに変わります。

---

## ZeroMapperの特徴

**CPU使用率ほぼ0%、メモリ使用量2MB。**

常駐アプリは使っていないときもリソースを消費すべきではありません。ZeroMapperはボタンを押していないときはポーリングを150msに落とし、画面ロックやスリープ中は完全に停止します。一日中起動したままでもバッテリーや動作に影響しません。

- インストール不要 — exeとconfig.tomlを同じフォルダに置くだけ
- フォアグラウンドウィンドウのタイトルでプロファイルを自動切替
- **ボタンコンボ** — `LB+A`や`RB+DpadLeft`のようなコンボを1つのキーボードショートカットにマッピング。ボタンが少ない小型コントローラーでも数十のショートカットをカバー
- タップ・ホールドの2種類の出力モード

---

## インストール

1. [Releases](../../releases) から最新のzipをダウンロード
2. 好きなフォルダに解凍（`ZeroMapper.exe`と`config.toml`が入っています）
3. `ZeroMapper.exe`を実行 → システムトレイにアイコンが表示されます

> **8BitDo XInputモードの設定**: **X**ボタンを押しながら**START**で電源を入れてください。モードスイッチがあるモデルは**X**の位置に合わせてください。

---

## 任天堂レイアウト（A↔B、X↔Yスワップ）

8BitDo ZeroやMicroなど任天堂配列のコントローラーは、XInputモードでボタン名が入れ替わります。

```
        Xbox配列                  任天堂配列（8BitDo）

         [Y]                        [X]
       [X] [B]                    [Y] [A]
         [A]                        [B]
```

同梱のconfigは**8BitDo Zero 2基準で任天堂レイアウト（`nintendo_layout = true`）がデフォルト**です。コントローラーに**印字されたボタン名**のままconfigに記述できます。

```toml
schema_version = 1
controller_player = 1
nintendo_layout = true   # 8BitDo Zero/Microなど任天堂配列コントローラー用
```

Xboxコントローラーを使う場合は`nintendo_layout = false`にするか、行を削除してください。

---

## デフォルトマッピング

同梱の`config.toml`にはClip Studio Paint、Photoshop、Aseprite、Kritaのプロファイルが含まれています。

### Clip Studio Paint

| ボタン | キー | 機能 |
|--------|------|------|
| A | P | ペン |
| Y | B | ブラシ |
| B | E | 消しゴム |
| X（ホールド） | Space | キャンバス移動 |
| DpadLeft | Ctrl+Z | 元に戻す |
| DpadRight | Ctrl+Y | やり直し |
| LB+Y | I | スポイト |
| LB+A | M | 選択ツール |
| LB+B | K | 塗りつぶし |
| RB+A | Ctrl+T | 変形 |
| Start | Ctrl+S | 保存 |

### Photoshop

| ボタン | キー | 機能 |
|--------|------|------|
| A | B | ブラシ |
| Y | I | スポイト |
| B | E | 消しゴム |
| X（ホールド） | Space | キャンバス移動 |
| DpadLeft | Ctrl+Z | 元に戻す |
| DpadRight | Ctrl+Shift+Z | やり直し |
| LB+A | M | 選択ツール |
| LB+B | V | 移動ツール |
| LB+DpadLeft | [ | ブラシサイズ縮小 |
| LB+DpadRight | ] | ブラシサイズ拡大 |
| RB+DpadLeft | Ctrl+- | 縮小表示 |
| RB+DpadRight | Ctrl+= | 拡大表示 |
| RB+A | Ctrl+T | 自由変形 |
| RB+B | X | 描画色/背景色の切替 |
| Start | Ctrl+S | 保存 |

### Aseprite

| ボタン | キー | 機能 |
|--------|------|------|
| A | B | ブラシ |
| B | E | 消しゴム |
| X（ホールド） | Space | キャンバス移動 |
| Y（ホールド） | Alt | スポイト |
| LB+Y（ホールド） | Ctrl | 複数選択 |
| DpadLeft | Ctrl+Z | 元に戻す |
| DpadRight | Ctrl+Y | やり直し |
| LB+A | M | 選択ツール |
| RB+X | Tab | UIの表示切替 |
| Start | Ctrl+S | 保存 |

### Krita

| ボタン | キー | 機能 |
|--------|------|------|
| A | B | ブラシ |
| B | E | 消しゴム |
| X（ホールド） | Space | キャンバス移動 |
| Y（ホールド） | Ctrl | ブラシサイズ変更 |
| DpadLeft | Ctrl+Z | 元に戻す |
| DpadRight | Ctrl+Shift+Z | やり直し |
| LB+A | Ctrl+R | 参照画像 |
| LB+B | V | 移動ツール |
| LB+DpadLeft | [ | ブラシサイズ縮小 |
| LB+DpadRight | ] | ブラシサイズ拡大 |
| RB+B | X | 描画色/背景色の切替 |
| RB+X | D | デフォルトカラー |
| RB+Y | F5 | ブラシ設定 |
| Start | Ctrl+S | 保存 |

---

## カスタマイズ

トレイメニュー → 「Open config.toml」で設定ファイルを開き、編集後に「Reload config」をクリックすれば即座に反映されます。再起動は不要です。

```toml
schema_version = 1
controller_player = 1  # XInputプレイヤー番号（1〜4）

[profiles.my_app]
name = "マイアプリ"           # トレイメニューに表示される名前
title_regex = "My App"       # ウィンドウタイトルに対してマッチング（正規表現可）

[profiles.my_app.map]
A = "P"                               # タップ: Aを押したらPを1回入力
X = { mode = "hold", send = "Space" } # ホールド: Xを押している間Spaceを維持
"LB+A" = "Ctrl+Z"                     # コンボ: LB+A同時押しでCtrl+Z
```

**コンボ優先ルール**: `LB+A`と`A`が両方マッピングされている場合、LB+Aを押すと`LB+A`のみが実行され、`A`は無視されます。

### 対応ボタン

`A` `B` `X` `Y` `LB` `RB` `LT` `RT` `Back` `Start` `L3` `R3` `DpadUp` `DpadDown` `DpadLeft` `DpadRight`

### 対応キー

`A`〜`Z`、`0`〜`9`、`F1`〜`F24`、`Ctrl`、`Alt`、`Shift`、`Win`、`Enter`、`Esc`、`Space`、`Tab`、`Backspace`、`Delete`、`Insert`、`Home`、`End`、`PageUp`、`PageDown`、`Up`、`Down`、`Left`、`Right`、`CapsLock`、`[` `]` `\` `-` `=` `,` `.` `/` `;` `'`

---

## ビルド

```
cargo build --release
```

バイナリ: `target\release\zero_mapper.exe`

---

## 制限事項

コントローラーの入力を遮断しません。キーボード入力を追加で送信する仕組みのため、元のXInput信号は他のアプリにもそのまま届きます。
