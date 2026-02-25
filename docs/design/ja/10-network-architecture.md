
# 10. ネットワークアーキテクチャ

ネットワーク層は固定実装ではなくプラガブルなパイプラインとして設計されている。これは3つのニーズに対応する：プロトコル進化（今日のHTTP/3、明日のHTTP/4）、デュアルユースの柔軟性（ブラウザはフルネットワーキングが必要、アプリモードは不要またはカスタムスキームが必要）、サードパーティミドルウェアの拡張性。

## 10.1 リソースローダー抽象化

すべてのリソース取得はResourceLoaderトレイトを経由し、カスタムURLスキームとプロトコル実装を可能にする：

```rust
pub trait ResourceLoader: Send + Sync {
    /// このローダーが処理するURLスキーム
    fn schemes(&self) -> &[&str];

    /// リソースを取得
    async fn fetch(&self, request: Request) -> Result<Response, FetchError>;
}
```

| ローダー | スキーム | 備考 |
| --- | --- | --- |
| HttpLoader | http://, https:// | 組み込み。hyper + rustls + h3。リダイレクト、Cookie、CORSを処理。 |
| FileLoader | file:// | 組み込み。ローカルファイルアクセス（ブラウザモードではサンドボックス化）。 |
| DataLoader | data: | 組み込み。インラインデータURI（base64/plaintext）。 |
| AppResourceLoader | app://, elidex:// | elidex-appのみ。開発者がカスタムリソース解決を提供。Tauriのtauri://スキームに相当。 |

アプリ開発者は起動時にカスタムローダーを登録する：

```rust
let app = elidex_app::App::new()
    .resource_loader(AppResourceLoader::new(|path| {
        // 埋め込みリソースからUIアセットを提供
        Assets::get(path).map(|a| Response::ok(a.data))
    }));
```

## 10.2 ネットワークミドルウェアパイプライン

すべてのリクエストとレスポンスはミドルウェアチェーンを通過し、コアネットワーキングコードを変更せずに検査、変更、ブロックを可能にする：

```rust
pub trait NetworkMiddleware: Send + Sync {
    fn name(&self) -> &str;

    /// リクエスト送信前にインターセプト
    fn on_request(&self, req: &mut Request) -> MiddlewareAction {
        MiddlewareAction::Continue
    }

    /// レスポンス受信後にインターセプト
    fn on_response(&self, req: &Request, res: &mut Response) -> MiddlewareAction {
        MiddlewareAction::Continue
    }
}

pub enum MiddlewareAction {
    Continue,                // 次のミドルウェアに渡す
    Block,                   // リクエスト/レスポンスを破棄
    Redirect(Url),           // 別のURLにリダイレクト
    MockResponse(Response),   // 合成レスポンスを返す（fetchをスキップ）
}
```

ミドルウェアパイプラインは順序付けされている。各ミドルウェアは先行するすべてのミドルウェアが処理した後のリクエスト/レスポンスを見る：

```
Request ─▶ [DevToolsロガー] ─▶ [コンテンツブロッカー] ─▶ [カスタムヘッダー] ─▶ HTTP Fetch
                                                                                │
Response ◀─ [DevToolsロガー] ◀─ [コンテンツフィルター] ◀─────────────────◀─┘
```

## 10.3 ミドルウェアユースケース

| ユースケース | 提供者 | 実装 |
| --- | --- | --- |
| ネットワーク監視（DevTools） | elidex-browser | on_requestとon_responseでリクエスト/レスポンスのタイミング、ヘッダー、ボディサイズをログ。DevToolsネットワークタブに供給。 |
| コンテンツブロッキング（広告・トラッカー） | サードパーティ/拡張 | フィルターリスト（例：EasyList形式）に対するURLパターンマッチング。マッチしたURLにMiddlewareAction::Blockを返す。Elidexはフックを提供、ブロッキングポリシーは外部。 |
| プライバシー保護 | サードパーティ/拡張 | トラッキングクエリパラメータ（utm_*、fbclid）を除去、Refererヘッダーを変更、既知のフィンガープリンティングエンドポイントをブロック。 |
| APIモック（テスト） | elidex-appデベロッパー | 開発/テスト中にマッチしたAPIエンドポイントにMockResponseを返す。実際のネットワーク呼び出し不要。 |
| リクエスト変更 | elidex-appデベロッパー | アプリケーションコードを変更せず、送信リクエストに認証ヘッダー、APIキー、カスタムヘッダーを注入。 |
| キャッシュオーバーライド | どちらも | カスタムキャッシュ戦略（例：アプリモードの積極的オフラインキャッシュ、開発用のキャッシュバイパス）。 |

## 10.4 設計原則：エンジン中立性

Elidexはコンテンツブロッキングポリシーをバンドルせず、ミドルウェアメカニズムのみを意図的に提供する。これは意識的な設計判断である：

**Elidexが提供するもの：** NetworkMiddlewareトレイト、パイプラインエグゼキュータ、リファレンスDevToolsロギングミドルウェア。

**Elidexが提供しないもの：** フィルターリスト、広告ブロッキングルール、トラッカーデータベース、何をブロックすべきかについてのいかなる意見も。これらは拡張とサードパーティミドルウェアクレートの領域である。

これにより、elidexはコンテンツブロッキングの政治的・法的複雑さから距離を保ちつつ、ユーザーと拡張開発者が選択したポリシーを実装する完全な権限を確保する。ミドルウェアAPIは単純なURLブロッキングから完全なリクエスト/レスポンス書き換えまで、あらゆるものをサポートする表現力を持つ。

## 10.5 HTTPクライアントアーキテクチャ

### 10.5.1 HttpTransportトレイト

HTTPクライアント実装はトレイトの背後に抽象化され、elidexを特定のHTTPライブラリから分離する。初期実装はhyper + rustls + h3を使用するが、トレイト境界により将来の置換がエンジンの他の部分に影響しない：

```rust
pub trait HttpTransport: Send + Sync {
    /// リクエストを送信しレスポンスヘッダを受信。
    /// ボディはインクリメンタル処理のためストリームとして返す。
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse, TransportError>;

    /// トランスポートが指定プロトコルバージョンをサポートするか確認。
    fn supports(&self, protocol: HttpProtocol) -> bool;

    /// 接続プール統計（DevToolsと診断用）。
    fn pool_stats(&self) -> ConnectionPoolStats;
}

pub enum HttpProtocol {
    Http11,
    H2,
    H3,
}
```

初期実装：

```rust
pub struct HyperTransport {
    http11_client: hyper::Client<HttpsConnector>,   // HTTP/1.1 + HTTP/2（ALPN経由）
    h3_client: h3::client::Connection,              // HTTP/3 (QUIC)
    pool: ConnectionPool,
}

impl HttpTransport for HyperTransport { /* ... */ }
```

将来hyperが後継に取って代わられた場合、`HyperTransport`のみが置換される。上位のすべて — ResourceLoader、ミドルウェアパイプライン、Rendererのfetchリクエスト — は`dyn HttpTransport`に対してプログラムしており影響を受けない。

### 10.5.2 プロトコルネゴシエーション

ElidexはHTTP/1.1、HTTP/2、HTTP/3を自動ネゴシエーションでサポートする：

```
Client ──── DNSルックアップ (DoH) ────▶ IPアドレス
  │
  ├── HTTPS試行 (TLS 1.3, ALPN: h2, http/1.1)
  │     ├── h2ネゴシエート → HTTP/2多重化接続
  │     └── http/1.1ネゴシエート → HTTP/1.1接続
  │
  ├── Alt-Svcヘッダでh3受信 → QUIC試行
  │     └── QUICハンドシェイク成功 → HTTP/3接続
  │
  └── フォールバック: HTTPS失敗かつユーザー/ポリシーが許可 → HTTP（警告付き）
```

| プロトコル | ステータス | 備考 |
| --- | --- | --- |
| HTTP/3 (QUIC) | 利用可能なら優先 | 低レイテンシ（0-RTT）、パケットロスの多いネットワークでの性能向上。Alt-SvcヘッダまたはDNS HTTPSレコードで発見。 |
| HTTP/2 | HTTPSのデフォルト | 単一TCP接続上の多重化ストリーム。HTTPレイヤーのヘッドオブラインブロッキングを排除。 |
| HTTP/1.1 | フォールバック | HTTP/2未対応サーバーに必要。多くの内部/レガシーサーバー。 |
| HTTP（平文） | 非推奨 | HTTPS-Onlyモードがデフォルト（10.6.2節参照）。平文HTTPはユーザーの明示的オプトインまたはHSTSプリロードミスが必要。 |

エンジン層のcore/compatとは異なり、レガシー機能をコンパイル時に除外できない — HTTPプロトコルサポートはサーバー能力に依存する。3つのプロトコルバージョンすべてが利用可能なまま、エンジンが自動的に最善のオプションを選択。

### 10.5.3 接続管理

Network Process（第5章）がすべてのタブで共有される集中型接続プールを管理する：

```rust
pub struct ConnectionPool {
    /// オリジンごとの接続上限
    max_per_origin: usize,          // デフォルト: HTTP/1.1は6、HTTP/2は1（多重化）
    max_total: usize,               // デフォルト: 256
    idle_timeout: Duration,          // デフォルト: 90秒
    /// HTTP/2の接続統合：TLS証明書を共有するオリジンは
    /// 同じ接続を再利用可能
    coalesce_h2: bool,              // デフォルト: true
}
```

| 機能 | 動作 |
| --- | --- |
| Keep-alive | 接続を`idle_timeout`まで保持。同一オリジンへの後続リクエストで再利用。 |
| HTTP/2多重化 | オリジンごとに単一TCP接続がすべての同時リクエストを多重化ストリームとして搬送。6接続上限を排除。 |
| 接続統合 | HTTP/2接続は同じTLS証明書とIPを共有するオリジン間で再利用可能。CDNの接続オーバーヘッドを削減。 |
| Preconnect | `<link rel="preconnect">`と`dns-prefetch`ヒントが早期接続セットアップをトリガー。最初のリクエスト前にDNS、TCP、TLSハンドシェイクを完了。 |

### 10.5.4 リクエスト優先度

HTTP/2とHTTP/3はストリーム優先度をサポートし、ブラウザがどのリソースが最も重要かをシグナルできる：

| 優先度 | リソース種別 | メカニズム |
| --- | --- | --- |
| 最高 | HTMLドキュメント、CSS（レンダリングブロッキング） | HTTP/2: PRIORITYフレーム、weight 256。HTTP/3: Urgency 0（Extensible Priorities）。 |
| 高 | JS（パーサーブロッキング）、Webフォント | Urgency 1–2。 |
| 中 | ファーストビュー画像、プリロードリソース | Urgency 3。 |
| 低 | ファーストビュー外画像、プリフェッチ | Urgency 4–5。`fetchpriority="low"`属性。 |
| 最低 | バックグラウンドfetch、アナリティクスビーコン | Urgency 6–7。 |

`fetchpriority` HTML属性とPriority Hints APIにより、Web開発者がデフォルト優先度をオーバーライド可能。

## 10.6 TLS & 証明書処理

### 10.6.1 TLS実装

TLSはRustネイティブのTLSライブラリであるrustlsで提供される。OpenSSLのC依存を回避し、ネットワークスタックで最もセキュリティクリティカルなコードにRustのメモリ安全性の恩恵を受ける：

| 側面 | 決定 | 根拠 |
| --- | --- | --- |
| ライブラリ | rustls + aws-lc-rs（暗号バックエンド） | Rustネイティブ。メモリ安全。aws-lc-rsがエンタープライズ向けFIPS検証済み暗号を提供。 |
| TLSバージョン | TLS 1.3優先、TLS 1.2サポート | TLS 1.2は依然広く必要（多くの企業環境、レガシーサーバー）。TLS 1.0/1.1は未サポート（非推奨）。 |
| 証明書検証 | webpki（Rustネイティブ） | OSトラストストアまたはバンドルされたMozillaルート証明書に対する標準X.509チェーン検証。 |
| Certificate Transparency | 公的に信頼された証明書に施行 | SCT（Signed Certificate Timestamp）検証。不正発行された証明書を検出。 |

### 10.6.2 HTTPS-Onlyモード

HTTPS-Onlyはelidex-browserのデフォルト。平文HTTPリクエストは自動的にアップグレードされる：

```
ユーザーが http://example.com にナビゲート
  → エンジンが https://example.com に書き換え
  → HTTPS接続成功 → 通常通り続行
  → HTTPS接続失敗 → インタースティシャル警告を表示
     → ユーザーがHTTPで続行をオプトイン可能（サイトごとの例外）
```

「セキュアバイデフォルト」の姿勢。エンジン層のcore/compatがパフォーマンスのためにレガシーを削除するのとは異なり、ここではレガシー（HTTP）を実用的互換性のために保持しつつ、明示的なユーザーアクションを要求する。

## 10.7 セキュリティファーストデフォルト

Elidexのネットワーク層はエンジンの段階的劣化と同じ哲学を適用する：セキュアなモダンパスがデフォルトで、レガシーは明示的フォールバックとして利用可能。

| 機能 | elidexデフォルト | 業界状況 | 設定 |
| --- | --- | --- | --- |
| HTTPS-Onlyモード | ON | Chrome/Firefox: デフォルトOFF、オプトイン | ユーザーがサイト単位またはグローバルで無効化可能 |
| DNS over HTTPS (DoH) | ON（Cloudflareまたはシステムリゾルバ） | Chrome: 地域により異なる。Firefox: 米国でON。 | リゾルバ設定可能。企業はポリシーで無効化可能。 |
| サードパーティCookie | ブロック | Chrome: 段階的廃止。Firefox: ブロック（ETP）。Safari: ブロック（ITP）。 | サイトがStorage Access APIで例外を要求可能 |
| HSTSプリロードリスト | バンドル、自動更新 | ブラウザ間で標準 | — |
| HSTS動的エントリ | 尊重、永続化 | 標準 | プロファイルごとの永続化 |
| Mixed content | ブロック（アクティブ）、アップグレード（パッシブ） | Chrome: パッシブ自動アップグレード。Firefox: 同様。 | アクティブmixed content（スクリプト、iframe）は常にブロック。パッシブ（画像）はHTTPSに自動アップグレード。 |

### 10.7.1 DNS解決

```rust
pub trait DnsResolver: Send + Sync {
    async fn resolve(&self, host: &str) -> Result<Vec<IpAddr>, DnsError>;
}

pub struct DohResolver {
    endpoint: Url,          // 例: https://1.1.1.1/dns-query
    fallback: Option<Box<dyn DnsResolver>>,  // フォールバックとしてのシステムリゾルバ
    cache: DnsCache,         // TTLベースのインメモリキャッシュ
}

pub struct SystemResolver;   // OS提供の解決（getaddrinfo）
```

| リゾルバ | デフォルト | 備考 |
| --- | --- | --- |
| DoH (DNS over HTTPS) | ブラウザのデフォルト | 暗号化DNSクエリ。ISP/ネットワークレベルのDNS操作を防止。 |
| システムリゾルバ | elidex-appのデフォルト、ブラウザのフォールバック | OS DNS設定を使用。/etc/resolv.conf、企業DNSを尊重。 |

DnsResolverトレイトにより、エンタープライズデプロイメントがカスタムリゾルバを注入可能（例：内部ドメイン解決を持つ企業DNS）。

## 10.8 CORS施行

Cross-Origin Resource SharingはRendererではなくNetwork Processで施行される。これにより侵害されたRendererでもCORSをバイパスできない：

```
RendererがFetchリクエストを送信
  → Network ProcessがRendererToNetwork::Fetchを受信
  → リクエストタイプを確認：
     ├── 同一オリジン → 続行
     ├── シンプルクロスオリジン → リクエスト送信、レスポンスのAccess-Control-Allow-Originを確認
     └── 非シンプルクロスオリジン → まずOPTIONS preflightを送信
         ├── preflight成功 → 実際のリクエストを送信
         └── preflight失敗 → CORSエラーをRendererに返す（レスポンスボディは非公開）
```

| CORS側面 | 実装 |
| --- | --- |
| Preflightキャッシュ | (origin, method, headers)タプルごとにキャッシュ。`Access-Control-Max-Age`ヘッダを尊重。Network Processメモリに格納。 |
| Credentialedリクエスト | `Access-Control-Allow-Credentials: true`が必要。`Access-Control-Allow-Origin`はワイルドカード不可。 |
| Opaqueレスポンス | `no-cors`モードがopaqueレスポンス（ステータス0、空ヘッダ/ボディ）をRendererに返す。ボディはService Workerキャッシュには利用可能だがスクリプトには不可。 |

## 10.9 Cookie管理

### 10.9.1 Cookie Jar

Cookie jarはNetwork Processに存在し、ネットワーク層でCookieポリシーを施行する：

```rust
pub struct CookieJar {
    store: CookieStore,              // オリジンごとのCookieストレージ
    policy: CookiePolicy,
}

pub struct CookiePolicy {
    block_third_party: bool,         // デフォルト: true
    same_site_default: SameSite,     // デフォルト: Lax（Chrome動作に一致）
    require_secure_for_same_site_none: bool,  // デフォルト: true
    partitioned_cookies: bool,       // CHIPSサポート。デフォルト: true
}
```

### 10.9.2 Cookie分類

| Cookieタイプ | デフォルト動作 | 備考 |
| --- | --- | --- |
| ファーストパーティ、SameSite=Lax | 許可 | 標準的なモダンCookie動作 |
| ファーストパーティ、SameSite=Strict | 許可 | より制限的 — クロスサイトナビゲーション時に送信されない |
| ファーストパーティ、SameSite=None; Secure | 許可 | Secure必須（HTTPSのみ） |
| サードパーティ | ブロック | デフォルト。サイトがStorage Access APIでアクセスを要求可能。 |
| Partitioned (CHIPS) | 許可 | トップレベルサイトごとのパーティショニング。プライバシー保護型サードパーティ状態。 |

### 10.9.3 セキュリティレスポンスヘッダ

Network Processがレスポンスをレンダラーに転送する前にセキュリティヘッダをパースし施行する：

| ヘッダ | 施行 |
| --- | --- |
| Content-Security-Policy (CSP) | ポリシーオブジェクトにパース。Rendererがインラインスクリプト/スタイルのブロック、eval制限、ソースアローリストを施行。違反はReporting API経由で報告。 |
| Strict-Transport-Security (HSTS) | 動的エントリを永続化層（第22章）に格納。以降のそのドメインへのリクエストは自動的にHTTPSにアップグレード。 |
| X-Frame-Options | ナビゲーション時に施行。DENY/SAMEORIGINがクロスオリジンページによるフレーミングを防止。CSP `frame-ancestors`に取って代わられたが依然サポート。 |
| Permissions-Policy | パース。ドキュメントとそのiframeで利用可能な機能（カメラ、マイク、geolocation）を制御。 |
| Cross-Origin-Opener-Policy (COOP) | ドキュメントが独自のブラウジングコンテキストグループを取得するか決定。SharedArrayBufferアクセスに必要。 |
| Cross-Origin-Embedder-Policy (COEP) | すべてのサブリソースがクロスオリジンローディングにオプトインすることを要求。SharedArrayBufferアクセスに必要。 |

## 10.10 レスポンス解凍

| エンコーディング | サポート | ライブラリ |
| --- | --- | --- |
| Brotli | コア | brotli（Rustネイティブ） |
| gzip / deflate | コア | flate2（Rustネイティブ） |
| zstd | コア | zstd（Rustバインディング） |

Accept-Encodingヘッダはサポートされるエンコーディングに基づき自動構築。解凍はストリーミング（レスポンスボディストリームと統合）で、レスポンス全体のメモリバッファリングを回避。

## 10.11 プロキシサポート

```rust
pub enum ProxyConfig {
    None,
    Http(Url),                    // HTTP CONNECTプロキシ
    Socks5(Url),                   // SOCKS5プロキシ
    Pac(Url),                      // PACスクリプトURL（自動設定）
    System,                        // OSプロキシ設定を使用
}
```

PACスクリプト評価はNetwork Process内のサンドボックス化されたJS環境（スクリプトエンジン、第14章）で実行される。企業プロキシ自動設定のための標準的なブラウザ要件。

## 10.12 elidex-appネットワーク設定

elidex-appモードでは、アプリケーション開発者がネットワークスタックをきめ細かく制御できる：

```rust
let app = elidex_app::App::new()
    // カスタムDNSリゾルバ（例：内部サービスディスカバリ）
    .dns_resolver(CustomDnsResolver::new())
    // サードパーティCookieブロックを無効化（アプリがすべてのコンテンツを制御）
    .cookie_policy(CookiePolicy { block_third_party: false, ..Default::default() })
    // 内部API用カスタムTLS証明書
    .add_root_certificate(internal_ca)
    // プロキシ設定
    .proxy(ProxyConfig::Http("http://proxy.internal:8080".parse().unwrap()))
    .build();
```

SingleProcessモード（第5章）では、Network「プロセス」はアプリケーションプロセス内のtokioタスクとなる。HttpTransportトレイト、ミドルウェアパイプライン、すべてのネットワーク設定は同一に動作する——唯一の違いはIPCがインプロセスチャネルに置き換わること。
