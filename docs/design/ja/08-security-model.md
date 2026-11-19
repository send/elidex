
# 8. セキュリティモデル

セキュリティ境界はプロジェクト開始時に確立しなければならない。アーキテクチャを根本的に制約するため。

## 8.1 プロセスサンドボックス

Renderer Processは最小限のシステムアクセスでサンドボックス化：

| プラットフォーム | サンドボックス機構 | 備考 |
| --- | --- | --- |
| Linux | seccomp-bpf + namespaces | Chromium実証済みアプローチ |
| macOS | App Sandbox (sandbox-exec) | ファイルシステム、ネットワークを制限 |
| Windows | Restricted tokens + job objects | Win32 APIサンドボックス |

Browser Processが特権ブローカーとして機能。Rendererはネットワークアクセス、ファイルI/O、クリップボード操作をIPC経由でリクエストし、Browser Processがポリシーを適用。

## 8.2 Webセキュリティポリシー

Same-Origin Policy、CORS、CSPはelidex-securityクレートで実装。CORSはNetwork Process（第10章§10.8）で施行、CSPはNetwork Processでパースされ、Rendererで施行。elidex-originクレートがオリジンの計算と比較を管理し、URLスキーム、ポート、opaqueオリジンの全複雑性を処理。

## 8.3 統一パーミッションモデル

Elidexはブラウザモードとアプリモードの両方で単一のパーミッション型を使用。パーミッションセットは同一で、付与メカニズムのみが異なる。

### 8.3.1 パーミッション型

```rust
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
pub enum Permission {
    Geolocation,
    Camera,
    Microphone,
    CameraAndMicrophone,
    Notifications,
    ClipboardRead,
    ClipboardWrite,
    Push,
    BackgroundSync,
    PersistentStorage,
    Midi,
    MidiSysex,
    Bluetooth,
    Usb,
    Serial,
    Hid,
    ScreenCapture,
    WindowManagement,
    LocalFonts,
    IdleDetection,
    /// Origin Private File System以外のファイルシステムアクセス
    FileSystemAccess,
    /// センサーAPI（accelerometer、gyroscope等）
    Sensors,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PermissionState {
    /// さらなるユーザーインタラクションなしでアクセス許可。
    Granted,
    /// アクセス拒否。ブラウザモードではユーザーが明示的に拒否。
    Denied,
    /// ユーザーにまだ確認していない。ブラウザモードのみ。
    /// パーミッションリクエストでユーザーに表示されるプロンプトをトリガー。
    Prompt,
}
```

### 8.3.2 付与メカニズム

| モード | デフォルト状態 | 付与メカニズム | 永続性 |
| --- | --- | --- | --- |
| ブラウザ | Prompt | ランタイムユーザープロンプト | オリジン単位でbrowser.sqlite（第22章） |
| アプリ | Denied | ビルド時マニフェスト / `App::grant()` | 静的（アプリにコンパイル） |

ブラウザモードでは、Webページがパーミッションゲート付きAPIを呼び出すと（例：`navigator.geolocation.getCurrentPosition()`）、パーミッションチェックがBrowser Processを経由：

```
Renderer（Webコンテンツがゲート付きAPIを呼出）
  │  IPC: PermissionRequest { origin, permission }
  ▼
Browser Process: PermissionManager
  │  1. オリジンレベルの保存済み決定を確認
  │     → GrantedまたはDenied：即座に返却
  │  2. Permissions-Policyを確認（ドキュメントレベル、§8.4）
  │     → ポリシーでブロック：Deniedを返却
  │  3. 状態がPrompt：BrowserShellにUIをリクエスト
  │     → BrowserShellがユーザーにプロンプト表示
  │     → ユーザーが許可または拒否
  │  4. オリジン単位で決定を保存
  │  IPC: PermissionResponse { state }
  ▼
Renderer（API呼び出しが成功または失敗）
```

アプリモードではプロンプトステップなし。パーミッションはビルド時に解決：

```rust
let app = elidex_app::App::new()
    .grant(Permission::Camera)
    .grant(Permission::Microphone)
    .grant(Permission::FileSystemAccess)
    .deny(Permission::Geolocation)
    .build();
    // 他のすべてのパーミッションはデフォルトDenied。
```

このケイパビリティベースモデルは、`nodeIntegration`がNode.jsランタイム全体をデフォルトでWebコンテンツに公開していたElectronの歴史的問題を回避。

### 8.3.3 PermissionManager

PermissionManagerはBrowser Process内に存在し、すべてのパーミッション決定の唯一の権限：

```rust
pub struct PermissionManager {
    /// 永続的なオリジン単位の決定（ブラウザモード）
    store: PermissionStore,
    /// 静的付与（アプリモード）
    app_grants: HashMap<Permission, PermissionState>,
    /// 一時的セッションオーバーライド（例：ワンタイム付与）
    session_overrides: HashMap<(Origin, Permission), PermissionState>,
}

impl PermissionManager {
    /// コアパーミッションチェック。ゲート付きAPI呼出ごとに呼ばれる。
    pub fn check(
        &self,
        origin: &Origin,
        permission: Permission,
        document_policy: &PermissionsPolicy,
        frame_policy: &FramePermissions,
    ) -> PermissionState {
        // レイヤー1: ドキュメントレベルのPermissions-Policyヘッダー
        if !document_policy.allows(permission) {
            return PermissionState::Denied;
        }

        // レイヤー2: フレームレベルのallow属性（iframe用）
        if !frame_policy.allows(permission) {
            return PermissionState::Denied;
        }

        // レイヤー3: オリジンレベルの決定
        if let Some(state) = self.session_overrides.get(&(origin.clone(), permission)) {
            return *state;
        }

        if let Some(state) = self.store.get(origin, permission) {
            return state;
        }

        // 保存済み決定なし
        if !self.app_grants.is_empty() {
            // アプリモード：静的付与を確認、デフォルトDenied
            self.app_grants.get(&permission).copied()
                .unwrap_or(PermissionState::Denied)
        } else {
            // ブラウザモード：Prompt
            PermissionState::Prompt
        }
    }

    /// ユーザー決定を保存（ブラウザモードのみ）。
    pub fn set_decision(
        &mut self,
        origin: &Origin,
        permission: Permission,
        state: PermissionState,
        duration: DecisionDuration,
    ) {
        match duration {
            DecisionDuration::Persistent => {
                self.store.set(origin, permission, state);
            }
            DecisionDuration::Session => {
                self.session_overrides.insert((origin.clone(), permission), state);
            }
        }
    }

    /// オリジンの全パーミッションをリセット（ユーザーアクション：「サイト設定のリセット」）。
    pub fn reset_origin(&mut self, origin: &Origin) {
        self.store.remove_origin(origin);
        self.session_overrides.retain(|(o, _), _| o != origin);
    }
}
```

### 8.3.4 パーミッションストレージ

パーミッション決定は`browser.sqlite`（第22章、ブラウザ所有データ）に格納：

```sql
CREATE TABLE permissions (
    origin     TEXT NOT NULL,
    permission TEXT NOT NULL,
    state      TEXT NOT NULL CHECK (state IN ('granted', 'denied')),
    granted_at INTEGER NOT NULL,  -- unixタイムスタンプ
    PRIMARY KEY (origin, permission)
);
```

プロファイルクリア（「サイトデータの消去」）で影響オリジンの全パーミッション行を削除。ブラウザ全体リセットでテーブルをtruncate。

### 8.3.5 Permissions API（Web）

`navigator.permissions` APIがパーミッション状態をWebコンテンツに公開：

```rust
// navigator.permissions.query({ name: "geolocation" })
pub async fn query_permission(
    &self,
    origin: &Origin,
    descriptor: PermissionDescriptor,
) -> PermissionStatus {
    let state = self.permission_manager.check(
        origin,
        descriptor.name,
        &self.document_policy,
        &self.frame_permissions,
    );

    PermissionStatus {
        state,
        // PermissionStatus.onchangeは状態変更時に発火
        // （例：ユーザーがブラウザ設定でパーミッションを取消）
        change_signal: self.watch(origin, descriptor.name),
    }
}
```

`PermissionStatus`の`onchange`イベントは、ユーザーがブラウザ設定経由でパーミッションを変更した時に発火。PermissionManagerが影響するオリジン+パーミッションのPermissionStatusを保持する全Renderer Processに変更をブロードキャスト。

## 8.4 Permissions-Policy

Permissions-Policy HTTPヘッダー（旧Feature-Policy）により、サーバーがドキュメントおよび埋め込みフレームで利用可能なパーミッションを制御。

### 8.4.1 ポリシーパース

```
Permissions-Policy: camera=(), microphone=(self "https://meet.example.com"), geolocation=*
```

このヘッダーの意味：cameraは完全にブロック、microphoneはドキュメント自身のオリジンと`https://meet.example.com`に許可、geolocationはすべてのオリジンに許可。

```rust
pub struct PermissionsPolicy {
    directives: HashMap<Permission, AllowList>,
}

pub enum AllowList {
    /// どのオリジンも許可されない。
    None,
    /// ドキュメント自身のオリジンのみ。
    SelfOnly,
    /// 特定のオリジン。
    Origins(Vec<Origin>),
    /// すべてのオリジン。
    All,
}
```

### 8.4.2 Iframe統合

`<iframe>`の`allow`属性がフレームレベルポリシーを提供：

```html
<iframe src="https://meet.example.com"
        allow="camera; microphone">
</iframe>
```

```rust
pub struct FramePermissions {
    /// 親フレームのallow属性で明示的に許可されたパーミッション。
    allowed: HashSet<Permission>,
    /// 親ドキュメントのPermissions-Policy（継承）。
    parent_policy: Arc<PermissionsPolicy>,
}

impl FramePermissions {
    pub fn allows(&self, permission: Permission) -> bool {
        // iframeが明示的に付与されかつ親ポリシーが許可
        self.allowed.contains(&permission)
            && self.parent_policy.allows_for_origin(permission, &self.frame_origin)
    }
}
```

Permissions-Policyはオリジンに保存済みの`Granted`決定があっても適用。サーバーがユーザーレベルの付与に関係なく特定ドキュメントのパーミッションを取り消せる。

### 8.4.3 デフォルトポリシー

ヘッダーで言及されないパーミッションにはデフォルトポリシーが適用。大半のパーミッションは`self`がデフォルト（ドキュメント自身のオリジンのみ許可、埋め込みiframeは不可）。一部パーミッションは`*`がデフォルト（例：一部ブラウザの`autoplay`）。ElidexはW3C Permissions-Policy仕様のデフォルトに従う。

## 8.5 パーミッションゲート付き機能

| パーミッション | API | 機密度 | 備考 |
| --- | --- | --- | --- |
| Geolocation | `navigator.geolocation` | 高 | 物理的位置追跡 |
| Camera | `getUserMedia({ video })` | 高 | ビデオキャプチャ |
| Microphone | `getUserMedia({ audio })` | 高 | オーディオキャプチャ |
| Notifications | `Notification`コンストラクタ | 中 | システムレベルアラート |
| ClipboardRead | `navigator.clipboard.read()` | 中 | クリップボード内容へのアクセス |
| ClipboardWrite | `navigator.clipboard.write()` | 低 | 大半のブラウザはユーザージェスチャで自動付与 |
| Push | `PushManager.subscribe()` | 中 | バックグラウンドメッセージング |
| PersistentStorage | `navigator.storage.persist()` | 低 | サイトデータのエビクション防止 |
| Midi | `navigator.requestMIDIAccess()` | 中 | MIDIデバイスアクセス |
| MidiSysex | `requestMIDIAccess({ sysex: true })` | 高 | システムエクスクルーシブMIDI（ファームウェア書込） |
| Bluetooth | `navigator.bluetooth.requestDevice()` | 高 | Bluetoothデバイスペアリング |
| Usb | `navigator.usb.requestDevice()` | 高 | USBデバイスアクセス |
| Serial | `navigator.serial.requestPort()` | 高 | シリアルポートアクセス |
| Hid | `navigator.hid.requestDevice()` | 高 | HIDデバイスアクセス |
| ScreenCapture | `getDisplayMedia()` | 高 | 画面/ウィンドウ録画 |
| LocalFonts | `queryLocalFonts()` | 中 | フィンガープリントベクター |
| IdleDetection | `IdleDetector.start()` | 中 | ユーザーアクティビティ監視 |
| Sensors | `Accelerometer`、`Gyroscope` | 中 | モーション/方向データ |

高機密度パーミッションはプロンプトトリガーにユーザージェスチャ（クリック、キー押下）が必要。ユーザージェスチャなしでリクエストはサイレントに拒否。

## 8.6 パーミッションプロンプトUI

パーミッションプロンプトはBrowserShell（第24章）の責務。PermissionManagerがシェルトレイト経由でプロンプトをリクエスト：

```rust
pub trait PermissionPrompter {
    /// ユーザーにパーミッションプロンプトを表示。
    /// ユーザーの決定、またはプロンプトが却下された場合はNoneを返す。
    async fn show_prompt(
        &self,
        request: PermissionPromptRequest,
    ) -> Option<PermissionDecision>;
}

pub struct PermissionPromptRequest {
    pub origin: Origin,
    pub permission: Permission,
    pub has_user_gesture: bool,
    pub favicon: Option<ImageData>,
}

pub struct PermissionDecision {
    pub state: PermissionState,
    pub duration: DecisionDuration,
}

pub enum DecisionDuration {
    /// 将来の訪問でも記憶
    Persistent,
    /// このセッションのみ（ブラウザ終了時にクリア）
    Session,
}
```

### 8.6.1 プロンプト動作

| 動作 | ルール |
| --- | --- |
| 一度に1プロンプト | 同時リクエストをキュー。複数プロンプトを重ねない。 |
| ユーザージェスチャ必須 | 高機密度パーミッションはユーザージェスチャが必要。ジェスチャなし → サイレント拒否。 |
| 却下 = Deny（セッション） | 選択せずプロンプトを閉じるとセッションスコープのDeny。次回訪問時に再試行可能。 |
| 繰り返し拒否 | 同一オリジン+パーミッションで3回連続拒否後、プロンプトなしで自動拒否。サイト設定でリセット。 |
| バックグラウンドタブ | バックグラウンドタブではプロンプト抑制。タブがフォーカスされるまでリクエストをキュー。 |

### 8.6.2 ネイティブ vs. セルフホストchrome

| Chromeモード | プロンプト実装 |
| --- | --- |
| ネイティブ（egui/iced、Phase 1–2） | プラットフォーム適切なダイアログまたはブラウザchrome内インラインバー。 |
| セルフホスト（HTML/CSS、Phase 3+） | ブラウザchromeドキュメント内のHTML描画プロンプト。スプーフィング防止のため、リクエスト元Rendererではなく Browser Processでレンダリング。 |

## 8.7 パーミッション取消

ユーザーは以下を通じてパーミッションを取消可能：

| メカニズム | スコープ |
| --- | --- |
| サイト情報パネル（南京錠アイコン） | オリジン単位、パーミッション単位 |
| ブラウザ設定 → サイトパーミッション | オリジン単位またはグローバル |
| 閲覧データの消去 | 影響オリジンの全パーミッション |

パーミッション取消時、PermissionManagerが影響するオリジン+パーミッションのPermissionStatusを保持する全Renderer Processに`PermissionChanged`イベントをブロードキャスト。パーミッションを使用中のアクティブセッション（例：進行中のgetUserMediaストリーム）は終了。

```rust
pub enum PermissionEvent {
    Changed {
        origin: Origin,
        permission: Permission,
        new_state: PermissionState,
    },
    OriginReset { origin: Origin },
}
```

## 8.8 アプリモードセキュリティ

### 8.8.1 ケイパビリティモデル

elidex-appモードではパーミッションはビルド時に宣言される静的ケイパビリティ。ユーザープロンプトなし：

```rust
let app = elidex_app::App::new()
    .grant(Permission::Camera)
    .grant(Permission::Microphone)
    .grant(Permission::FileSystemAccess)
    .deny(Permission::Geolocation)
    // 他のすべてのパーミッションはデフォルトDenied
    .build();
```

概念的にはAndroidの`AndroidManifest.xml`パーミッションやmacOS App Sandboxエンタイトルメントに類似するが、OSレベルではなくエンジンレベルで適用。

### 8.8.2 拡張アプリケイパビリティ

アプリモードはWebパーミッション以外の追加ケイパビリティをサポート。ローカルインストールアプリケーションの高い信頼度に対応：

```rust
pub enum AppCapability {
    /// 全Web標準パーミッション
    WebPermission(Permission),

    /// OPFS以外のファイルシステムアクセス
    FileRead(PathPattern),
    FileWrite(PathPattern),

    /// 無制限ネットワークアクセス（CORSバイパス）
    NetworkUnrestricted,

    /// システムコマンドへのアクセス
    ProcessSpawn(Vec<String>),

    /// 環境変数へのアクセス
    EnvRead(Vec<String>),

    /// 他のelidex-appインスタンスやネイティブプロセスとのIPC
    Ipc,
}
```

各拡張ケイパビリティは操作実行前にBrowser Process（またはアプリモードのマージ済みシングルプロセス）でチェック。未宣言のケイパビリティはランタイムでエラーと共に拒否。

### 8.8.3 ケイパビリティ監査

アプリ起動時、すべての付与済みケイパビリティがログに記録。デバッグビルドでは、ケイパビリティチェックがtracingスパンを発行し、開発者がアプリが宣言したパーミッションのみを使用していることを検証可能。

```rust
impl CapabilityChecker {
    pub fn check(&self, capability: &AppCapability) -> Result<(), CapabilityError> {
        if self.is_granted(capability) {
            tracing::trace!(?capability, "capability check passed");
            Ok(())
        } else {
            tracing::warn!(?capability, "capability denied — not granted in manifest");
            Err(CapabilityError::NotGranted(capability.clone()))
        }
    }
}
```
