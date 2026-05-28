# SurrealDB JavaScript SDK v2.0.3 — Comprehensive Reference

> **Source:** Official TypeScript definitions from `surrealdb@2.0.3` (jsDelivr CDN) and SurrealDB documentation at `surrealdb.com/docs/languages/javascript/`.

---

## Table of Contents

1. [Installation & Import](#1-installation--import)
2. [Connecting to SurrealDB](#2-connecting-to-surrealdb)
3. [Authentication](#3-authentication)
4. [The `Surreal` Class](#4-the-surreal-class)
5. [Query Methods (SurrealQueryable)](#5-query-methods-surrealqueryable)
6. [Query Builder Classes](#6-query-builder-classes)
7. [Value Types](#7-value-types)
8. [Utility Functions](#8-utility-functions)
9. [Expression Builder](#9-expression-builder)
10. [TypeScript Types](#10-typescript-types)
11. [Error Classes](#11-error-classes)
12. [Live Queries & Subscriptions](#12-live-queries--subscriptions)
13. [Engine & Codec Interfaces](#13-engine--codec-interfaces)
14. [Diagnostics](#14-diagnostics)

---

## 1. Installation & Import

```bash
npm install surrealdb
# or
pnpm add surrealdb
# or
yarn add surrealdb
# or
bun add surrealdb
```

### Import

```ts
// ESM / TypeScript
import Surreal from "surrealdb";
// or named import
import { Surreal } from "surrealdb";

// CommonJS
const { Surreal } = require("surrealdb");

// CDN (unpkg / jsdelivr)
import Surreal from "https://unpkg.com/surrealdb";
```

### Peer Dependencies

- `typescript ^5.0.0`
- `tslib ^2.6.3`

---

## 2. Connecting to SurrealDB

### Creating a Connection

```ts
const db = new Surreal(options?: DriverOptions);
await db.connect(url: string | URL, opts?: ConnectOptions);
```

### Supported Protocols

| Protocol | Description |
|----------|-------------|
| `ws://` / `wss://` | WebSocket (long-lived, stateful) |
| `http://` / `https://` | HTTP (short-lived, stateless) |
| `mem://` | In-memory (WASM engine) |
| `indxdb://` | IndexedDB (browser WASM) |
| `rocksdb://` | File-system (Node.js engine) |

### Connection Options (`ConnectOptions`)

```ts
interface ConnectOptions {
  namespace?: string;
  database?: string;
  authentication?: AuthProvider;  // static or async function
  versionCheck?: boolean;         // default: true
  invalidateOnExpiry?: boolean;   // default: false
  reconnect?: boolean | Partial<ReconnectOptions>;  // default: true
}
```

### Reconnect Options (`ReconnectOptions`)

```ts
interface ReconnectOptions {
  enabled: boolean;
  attempts: number;               // -1 for unlimited
  retryDelay: number;             // ms
  retryDelayMax: number;          // ms
  retryDelayMultiplier: number;
  retryDelayJitter: number;       // float percentage
  catch?: (error: Error) => boolean;
}
```

### Connection Status

```ts
type ConnectionStatus = "disconnected" | "connecting" | "reconnecting" | "connected";

db.status       // Get current status
db.isConnected  // Boolean shortcut
db.ready        // Promise that resolves when connected
```

### Events

```ts
db.subscribe("connecting", () => {});
db.subscribe("connected", (url: string) => {});
db.subscribe("reconnecting", () => {});
db.subscribe("disconnected", () => {});
db.subscribe("error", (error: Error) => {});
db.subscribe("auth", (tokens: Tokens | null, session: Session) => {});
db.subscribe("using", (nsDb: NamespaceDatabase, session: Session) => {});
```

### Closing

```ts
await db.close();
```

---

## 3. Authentication

### Auth Types

```ts
// System users
type RootAuth = { username: string; password: string };
type NamespaceAuth = { namespace: string; username: string; password: string };
type DatabaseAuth = { namespace: string; database: string; username: string; password: string };

// Access method authentication
type AccessSystemAuth = {
  namespace?: string; database?: string;
  username: string; password: string; access: string;
};
type AccessBearerAuth = {
  namespace?: string; database?: string;
  access: string; key: string;
};
type AccessRecordAuth = {
  namespace?: string; database?: string;
  access: string;
  variables: { ns?: never; db?: never; ac?: never; [K: string]: unknown };
};

type SystemAuth = RootAuth | NamespaceAuth | DatabaseAuth;
type AccessAuth = AccessSystemAuth | AccessBearerAuth | AccessRecordAuth;
type AnyAuth = SystemAuth | AccessAuth;

type Token = string;
type Tokens = { access: Token; refresh?: Token };
type AuthProvider = SystemAuth | Token | null | ((session: Session) => ProvidedAuth | Promise<ProvidedAuth>);
```

### Methods

```ts
// Sign in as any user type
const tokens = await db.signin(auth: AnyAuth): Promise<Tokens>;

// Sign up as a record user
const tokens = await db.signup(auth: AccessRecordAuth): Promise<Tokens>;

// Authenticate with existing token(s)
await db.authenticate(token: Token | Tokens): Promise<Tokens>;

// Invalidate current auth
await db.invalidate(): Promise<void>;

// Get current access token
db.accessToken  // string | undefined
```

### Auth on Connect (Preferred for System Users)

```ts
await db.connect("ws://localhost:8000", {
  namespace: "myns",
  database: "mydb",
  authentication: { username: "root", password: "root" },
  // or a dynamic function:
  authentication: async () => ({ username: await getUsername(), password: await getPassword() }),
});
```

### Auth Events

```ts
db.subscribe("auth", (tokens: Tokens | null) => {
  if (tokens) console.log("Authenticated:", tokens.access);
  else console.log("Signed out");
});
```

---

## 4. The `Surreal` Class

`Surreal extends SurrealSession implements EventPublisher<SurrealEvents>`

### Constructor

```ts
constructor(options?: DriverOptions);
```

### Driver Options (`DriverOptions`)

```ts
interface DriverOptions {
  engines?: Engines;
  codecs?: Codecs;
  codecOptions?: CodecOptions;
  websocketImpl?: typeof WebSocket;
  fetchImpl?: typeof fetch;
}

interface CodecOptions {
  useNativeDates?: boolean;
  valueEncodeVisitor?: (value: unknown) => unknown;
  valueDecodeVisitor?: (value: unknown) => unknown;
}
```

### Connection Methods

```ts
connect(url: string | URL, opts?: ConnectOptions): Promise<true>;
close(): Promise<true>;
health(): Promise<void>;
version(): Promise<VersionInfo>;
ping(): Promise<true>;  // alias for health()
isFeatureSupported(feature: Feature): boolean;
```

### Session Management

```ts
status: ConnectionStatus;
isConnected: boolean;
ready: Promise<void>;
sessions(): Promise<Uuid[]>;
newSession(): Promise<SurrealSession>;
closeSession(): Promise<void>;
```

### Data Methods (inherited from SurrealQueryable via SurrealSession)

See [Section 5 — Query Methods](#5-query-methods-surrealqueryable).

### Export/Import

```ts
import(input: string | Blob | ReadableStream): Promise<void>;
export(options?: Partial<SqlExportOptions>): ExportPromise;
exportModel(name: string, version: string): ExportModelPromise;
```

### `SurrealSession` Class

```ts
class SurrealSession extends SurrealQueryable {
  namespace: string | undefined;
  database: string | undefined;
  accessToken: string | undefined;
  parameters: Record<string, unknown>;
  session: Session;
  isValid: boolean;

  forkSession(): Promise<SurrealSession>;
  closeSession(): Promise<void>;
  beginTransaction(): Promise<SurrealTransaction>;
  use(what?: Nullable<NamespaceDatabase>): Promise<NamespaceDatabase>;
  signup(auth: AccessRecordAuth): Promise<Tokens>;
  signin(auth: AnyAuth): Promise<Tokens>;
  authenticate(token: Token | Tokens): Promise<Tokens>;
  set(variable: string, value: unknown): Promise<void>;
  unset(variable: string): Promise<void>;
  invalidate(): Promise<void>;
  reset(): Promise<void>;

  subscribe<K extends keyof SessionEvents>(event: K, listener: (...payload: SessionEvents[K]) => void): () => void;

  static of(parent: SurrealSession, id: Session): SurrealSession;
}
```

### `SurrealTransaction` Class

```ts
class SurrealTransaction extends SurrealQueryable {
  commit(): Promise<void>;
  cancel(): Promise<void>;
}
```

---

## 5. Query Methods (SurrealQueryable)

These methods are available on `Surreal`, `SurrealSession`, and `SurrealTransaction`.

### `query()`

```ts
query<R extends unknown[] = unknown[]>(query: string, bindings?: Record<string, unknown>): Query<R>;
query<R extends unknown[] = unknown[]>(query: BoundQuery<R>): Query<R>;
```

Returns a `Query` instance that can be:
- **Awaited** directly (collects all results)
- Chained with `.json()`, `.collect(indexes...)`, `.stream()`, `.responses(indexes...)`

### `select()`

```ts
select<T>(recordId: AnyRecordId): SelectPromise<RecordResult<T> | undefined, T>;
select<T>(range: RecordIdRange): SelectPromise<RecordResult<T>[], T>;
select<T>(table: Table): SelectPromise<RecordResult<T>[], T>;
```

### `create()`

```ts
create<T>(recordId: AnyRecordId): CreatePromise<RecordResult<T>, T>;
create<T>(table: Table): CreatePromise<RecordResult<T>[], T>;
```

### `update()`

```ts
update<T>(recordId: AnyRecordId): UpdatePromise<RecordResult<T>, T>;
update<T>(range: RecordIdRange): UpdatePromise<RecordResult<T>[], T>;
update<T>(table: Table): UpdatePromise<RecordResult<T>[], T>;
```

### `upsert()`

```ts
upsert<T>(recordId: AnyRecordId): UpsertPromise<RecordResult<T>, T>;
upsert<T>(range: RecordIdRange): UpsertPromise<RecordResult<T>[], T>;
upsert<T>(table: Table): UpsertPromise<RecordResult<T>[], T>;
```

### `insert()`

```ts
insert<T>(data: Values<T> | Values<T>[]): InsertPromise<RecordResult<T>[]>;
insert<T>(table: Table, data: Values<T> | Values<T>[]): InsertPromise<RecordResult<T>[]>;
```

### `delete()`

```ts
delete<T>(recordId: AnyRecordId): DeletePromise<RecordResult<T>>;
delete<T>(range: RecordIdRange): DeletePromise<RecordResult<T>[]>;
delete<T>(table: Table): DeletePromise<RecordResult<T>[]>;
```

### `relate()`

```ts
relate<T>(from: AnyRecordId, edge: Table | RecordId, to: AnyRecordId, data?: Values<T>): RelatePromise<T>;
relate<T>(from: AnyRecordId[], edge: Table, to: AnyRecordId[], data?: Partial<T>): RelatePromise<T[]>;
```

### `run()`

```ts
run<T>(name: string, args?: unknown[]): RunPromise<T>;
run<T>(name: string, version: string, args?: unknown[]): RunPromise<T>;
```

### `live()`

```ts
live<T>(what: LiveResource): ManagedLivePromise<T>;
liveOf(id: Uuid): UnmanagedLivePromise;
```

### `auth()`

```ts
auth<T>(): AuthPromise<RecordResult<T> | undefined>;
```

### `api()`

```ts
api<TPaths = DefaultPaths>(prefix?: string): SurrealApi<TPaths>;
```

---

## 6. Query Builder Classes

All query builders extend `DispatchedPromise<T>` and can be **awaited** directly. They also offer chainable configuration methods.

### Common Methods (on most builders)

- `.json()` — Return results as JSON-compatible structure (loses SurrealQL type info)
- `.compile()` — Compile to a `BoundQuery`
- `.stream()` — Return `AsyncIterable<Frame<T, J>>`

### `SelectPromise<T, I, J>`

```ts
class SelectPromise<T, I, J> extends DispatchedPromise<MaybeJsonify<T, J>> {
  json(): SelectPromise<T, I, true>;
  fields(...fields: Field<I>[]): this;
  value(field: Field<I>): this;
  start(start: number): this;
  limit(limit: number): this;
  where(expr: ExprLike): this;
  fetch(...fields: Field<I>[]): this;
  timeout(timeout: Duration): this;
  version(version: DateTime): this;
  compile(): BoundQuery<[T]>;
  stream(): AsyncIterable<Frame<T, J>>;
}
```

### `CreatePromise<T, I, J>`

```ts
class CreatePromise<T, I, J> extends DispatchedPromise<MaybeJsonify<T, J>> {
  json(): CreatePromise<T, I, true>;
  content(data: Values<I>): this;
  patch(data: Patch[]): this;
  output(output: Output): this;
  timeout(timeout: Duration): this;
  version(version: DateTime): this;
  compile(): BoundQuery<[T]>;
  stream(): AsyncIterable<Frame<T, J>>;
}
```

### `UpdatePromise<T, I, J>`

```ts
class UpdatePromise<T, I, J> extends DispatchedPromise<MaybeJsonify<T, J>> {
  json(): UpdatePromise<T, I, true>;
  content(data: Values<I>): this;
  merge(data: Values<I>): this;
  replace(data: Values<I>): this;
  patch(data: Patch[]): this;
  where(expr: ExprLike): this;
  output(output: Output): this;
  timeout(timeout: Duration): this;
  compile(): BoundQuery<[T]>;
  stream(): AsyncIterable<Frame<T, J>>;
}
```

### `UpsertPromise<T, I, J>`

```ts
class UpsertPromise<T, I, J> extends DispatchedPromise<MaybeJsonify<T, J>> {
  json(): UpsertPromise<T, I, true>;
  content(data: Values<I>): this;
  merge(data: Values<I>): this;
  replace(data: Values<I>): this;
  patch(data: Patch[]): this;
  where(expr: ExprLike): this;
  output(output: Output): this;
  timeout(timeout: Duration): this;
  compile(): BoundQuery<[T]>;
  stream(): AsyncIterable<Frame<T, J>>;
}
```

### `InsertPromise<T, J>`

```ts
class InsertPromise<T, J> extends DispatchedPromise<MaybeJsonify<T, J>> {
  json(): InsertPromise<T, true>;
  relation(): this;
  ignore(): this;
  output(output: Output): this;
  timeout(timeout: Duration): this;
  version(version: DateTime): this;
  compile(): BoundQuery<[T]>;
  stream(): AsyncIterable<Frame<T, J>>;
}
```

### `DeletePromise<T, J>`

```ts
class DeletePromise<T, J> extends DispatchedPromise<MaybeJsonify<T, J>> {
  json(): DeletePromise<T, true>;
  output(output: Output): this;
  timeout(timeout: Duration): this;
  version(version: DateTime): this;
  compile(): BoundQuery<[T]>;
  stream(): AsyncIterable<Frame<T, J>>;
}
```

### `RelatePromise<T, J>`

```ts
class RelatePromise<T, J> extends DispatchedPromise<MaybeJsonify<T, J>> {
  json(): RelatePromise<T, true>;
  unique(): this;
  output(output: Output): this;
  timeout(timeout: Duration): this;
  version(version: DateTime): this;
  compile(): BoundQuery<[T]>;
  stream(): AsyncIterable<Frame<T, J>>;
}
```

### `Query<R, J>`

```ts
class Query<R extends unknown[], J> extends DispatchedPromise<Collect<R, J>> {
  get inner(): BoundQuery;
  json(): Query<R, true>;
  collect<T extends unknown[] = R>(...queries: number[]): Promise<Collect<T, J>>;
  stream<T = unknown>(): AsyncIterable<Frame<T, J>>;
  responses<T extends unknown[] = R>(...queries: number[]): Promise<Responses<T, J>>;
}
```

### `RunPromise<T, J>`

```ts
class RunPromise<T, J> extends DispatchedPromise<MaybeJsonify<T, J>> {
  json(): RunPromise<T, true>;
  compile(): BoundQuery<[T]>;
  stream(): AsyncIterable<Frame<T, J>>;
}
```

### `AuthPromise<T, J>`

```ts
class AuthPromise<T, J> extends DispatchedPromise<MaybeJsonify<T, J>> {
  json(): AuthPromise<T, true>;
  compile(): BoundQuery<[T]>;
  stream(): AsyncIterable<Frame<T, J>>;
}
```

### `ApiPromise<Req, Res, V, J>`

```ts
class ApiPromise<Req, Res, V, J> extends DispatchedPromise<Collect$1<Res, V, J>> {
  json(): ApiPromise<Req, Res, true>;
  header(name: string, value: string): this;
  query(name: string, value: string): this;
  value(): ApiPromise<Req, Res, true, J>;
  compile(): BoundQuery<[ApiResponse<Res>]>;
  stream(): AsyncIterable<Frame<ApiResponse<Res>, J>>;
}
```

### `ManagedLivePromise<T>` and `UnmanagedLivePromise`

```ts
class ManagedLivePromise<T> extends DispatchedPromise<LiveSubscription> {
  diff(): this;
  fields(...fields: Field<T>[]): this;
  value(field: Field<T>): this;
  where(expr: ExprLike): this;
  fetch(...fields: Field<T>[]): this;
  compile(): BoundQuery<[T]>;
}

class UnmanagedLivePromise extends DispatchedPromise<LiveSubscription> {
  // no chainable config — takes existing UUID
}
```

### `ExportPromise<R>` and `ExportModelPromise<R>`

```ts
class ExportPromise<R extends boolean = false> extends DispatchedPromise<ExportResult<string, R>> {
  raw(): ExportPromise<true>;
}
class ExportModelPromise<R extends boolean = false> extends DispatchedPromise<ExportResult<Uint8Array, R>> {
  raw(): ExportModelPromise<true>;
}
```

---

## 7. Value Types

All value types extend the abstract `Value` class:

```ts
abstract class Value {
  abstract equals(other: unknown): boolean;
  abstract toJSON(): unknown;
  abstract toString(): string;
}
```

### `RecordId<Tb, Id>`

```ts
class RecordId<Tb extends string = string, Id extends RecordIdValue = RecordIdValue> extends Value {
  constructor(table: Tb | Table<Tb>, id: Id);
  get table(): Table<Tb>;
  get id(): Id;
  equals(other: unknown): boolean;
  toJSON(): string;
  toString(): string;  // e.g. "person:123"
}

type RecordIdValue = string | number | Uuid | bigint | unknown[] | Record<string, unknown>;
type AnyRecordId = RecordId | StringRecordId;
```

### `StringRecordId`

```ts
class StringRecordId extends Value {
  constructor(rid: string | StringRecordId | RecordId);
  equals(other: unknown): boolean;
  toJSON(): string;
  toString(): string;
}
```

### `RecordIdRange<Tb, Id>`

```ts
class RecordIdRange<Tb, Id> extends Value {
  constructor(table: Tb | Table<Tb>, beg: Bound<Id>, end: Bound<Id>);
  get table(): Table<Tb>;
  get begin(): Bound<Id>;
  get end(): Bound<Id>;
  equals(other: unknown): boolean;
  toJSON(): string;
  toString(): string;
}
```

### `Table<Tb>`

```ts
class Table<Tb extends string = string> extends Value {
  constructor(tb: Tb);
  get name(): Tb;
  equals(other: unknown): boolean;
  toJSON(): string;
  toString(): string;
}
```

### `Uuid`

```ts
class Uuid extends Value {
  constructor(uuid: Uuid | UUID | string | ArrayBuffer | Uint8Array);
  equals(other: unknown): boolean;
  toJSON(): string;
  toString(): string;
  toUint8Array(): Uint8Array;
  toBuffer(): ArrayBufferLike;
  static v4(): Uuid;
  static v7(): Uuid;
}
```

### `Duration`

```ts
class Duration extends Value {
  constructor(input: Duration | DurationTuple | string);

  // Arithmetic
  add(other: Duration): Duration;
  sub(other: Duration): Duration;
  mul(factor: number | bigint): Duration;
  div(divisor: Duration): bigint;
  div(divisor: number | bigint): Duration;
  mod(mod: Duration): Duration;

  // Accessors (all return bigint)
  get nanoseconds(): bigint;
  get microseconds(): bigint;
  get milliseconds(): bigint;
  get seconds(): bigint;
  get minutes(): bigint;
  get hours(): bigint;
  get days(): bigint;
  get weeks(): bigint;
  get years(): bigint;

  // Serialization
  toCompact(): DurationTuple;
  toString(): string;
  toJSON(): string;

  // Static constructors
  static nanoseconds(ns: number | bigint): Duration;
  static microseconds(µs: number | bigint): Duration;
  static milliseconds(ms: number | bigint): Duration;
  static seconds(s: number | bigint): Duration;
  static minutes(m: number | bigint): Duration;
  static hours(h: number | bigint): Duration;
  static days(d: number | bigint): Duration;
  static weeks(w: number | bigint): Duration;
  static years(y: number | bigint): Duration;

  // Parsing
  static parseString(input: string): [bigint, bigint];
  static parseFloat(input: string): Duration;

  // Timing
  static measure(): () => Duration;
}

type DurationTuple = [number | bigint, number | bigint] | [number | bigint] | [];
```

### `DateTime`

```ts
class DateTime extends Value {
  constructor();  // current time
  constructor(input: DateTime | Date | DateTimeTuple | string | number | bigint);

  // Arithmetic
  add(duration: Duration): DateTime;
  sub(duration: Duration): DateTime;
  diff(other: DateTime): Duration;
  compare(other: DateTime): number;  // -1, 0, or 1

  // Accessors
  get nanoseconds(): bigint;
  get microseconds(): bigint;
  get milliseconds(): number;
  get seconds(): number;

  // Serialization
  toISOString(): string;
  toDate(): Date;
  toCompact(): [bigint, bigint];
  toString(): string;
  toJSON(): string;

  // Static constructors
  static now(): DateTime;
  static epoch(): DateTime;
  static fromEpochNanoseconds(ns: number | bigint): DateTime;
  static fromEpochMicroseconds(µs: number | bigint): DateTime;
  static fromEpochMilliseconds(ms: number | bigint): DateTime;
  static fromEpochSeconds(s: number | bigint): DateTime;

  static parseString(input: string): [bigint, bigint];
}
```

### `Decimal`

```ts
class Decimal extends Value {
  constructor(input: Decimal | string | number | bigint | DecimalTuple);

  // Arithmetic
  add(other: Decimal): Decimal;
  sub(other: Decimal): Decimal;
  mul(other: Decimal): Decimal;
  div(other: Decimal): Decimal;
  mod(other: Decimal): Decimal;
  abs(): Decimal;
  neg(): Decimal;

  // Accessors
  get int(): bigint;
  get frac(): bigint;
  get scale(): number;

  // Comparison
  isZero(): boolean;
  isNegative(): boolean;
  compare(other: Decimal): number;

  // Formatting
  round(precision: number): Decimal;
  toFixed(precision: number): string;
  toFloat(): number;
  toBigInt(): bigint;
  toScientific(): string;
  toParts(): { int: bigint; frac: bigint; scale: number };
  toString(): string;
  toJSON(): string;

  static fromScientificNotation(input: string): Decimal;
}

type DecimalTuple = [bigint, bigint, number];  // [int, frac, scale]
```

### `Range<Beg, End>`

```ts
class Range<Beg, End> extends Value {
  constructor(beg: Bound<Beg>, end: Bound<End>);
  get begin(): Bound<Beg>;
  get end(): Bound<End>;
  equals(other: unknown): boolean;
  toJSON(): string;
  toString(): string;
}
```

### Bound Types

```ts
class BoundIncluded<T> { constructor(value: T); readonly value: T; }
class BoundExcluded<T> { constructor(value: T); readonly value: T; }
type Bound<T> = BoundIncluded<T> | BoundExcluded<T> | undefined;
```

### `FileRef`

```ts
class FileRef extends Value {
  constructor(bucket: string, key: string);
  get bucket(): string;
  get key(): string;
  equals(other: unknown): boolean;
  toJSON(): string;
  toString(): string;
}
```

### `Future` (deprecated — removed in SurrealDB 3.0)

```ts
class Future extends Value {
  constructor(body: string);
  get body(): string;
}
```

### Geometry Types

```ts
abstract class Geometry extends Value {
  abstract toJSON(): GeoJson;
  abstract is(geometry: Geometry): boolean;
  abstract clone(): Geometry;
}

class GeometryPoint extends Geometry {
  readonly point: [number, number];
  constructor(point: [number | Decimal, number | Decimal] | GeometryPoint);
}
class GeometryLine extends Geometry {
  readonly line: [GeometryPoint, GeometryPoint, ...GeometryPoint[]];
  close(): void;
}
class GeometryPolygon extends Geometry {
  readonly polygon: [GeometryLine, ...GeometryLine[]];
}
class GeometryMultiPoint extends Geometry {
  readonly points: [GeometryPoint, ...GeometryPoint[]];
}
class GeometryMultiLine extends Geometry {
  readonly lines: [GeometryLine, ...GeometryLine[]];
}
class GeometryMultiPolygon extends Geometry {
  readonly polygons: [GeometryPolygon, ...GeometryPolygon[]];
}
class GeometryCollection extends Geometry {
  readonly collection: [Geometry, ...Geometry[]];
}

// GeoJSON types (Point, LineString, Polygon, MultiPoint, MultiLineString, MultiPolygon, GeometryCollection)
```

---

## 8. Utility Functions

### Template Literal Tags

```ts
// Create a BoundQuery (preferred over raw strings)
surql`SELECT * FROM person WHERE name = ${name}`  // returns BoundQuery

// Parse strings into SurrealQL values
s`string`           // returns string
d`2024-01-01`       // returns DateTime
r`person:123`       // returns StringRecordId
u`uuid-string`      // returns Uuid
```

### `BoundQuery` Class

```ts
class BoundQuery<R extends unknown[] = unknown[]> {
  constructor();
  constructor(origin: BoundQuery<R>);
  constructor(query: string, bindings?: Record<string, unknown>);

  get query(): string;
  get bindings(): Record<string, unknown>;

  append(other: BoundQuery<R>): this;
  append(query: string, bindings?: Record<string, unknown>): this;
  append(strings: TemplateStringsArray, ...values: unknown[]): this;
}
```

### `mergeBindings()`

```ts
mergeBindings(target: Record<string, unknown>, source: Record<string, unknown>): void;
```

### JSON Conversion

```ts
jsonify<T>(input: T): Jsonify<T>;                     // Recursively convert to JSON-safe
toSurqlString(input: unknown): string;                 // Convert to SurrealQL string repr
```

### Equality

```ts
equals(x: unknown, y: unknown): boolean;  // Deep recursive comparison of SurrealQL values
```

### Escape Functions

```ts
escapeIdent(str: string): string;         // Escape for SurrealQL ident (e.g. column name)
escapeNumber(num: number | bigint): string; // Escape for SurrealQL ident from number
escapeIdPart(id: RecordIdValue): string;  // Escape record ID value part
escapeRangeBound<T>(bound: Bound<T>): string; // Escape range bound value
```

### Type Checking

```ts
isLiveResult(v: unknown): v is LiveResult;
isVersionSupported(version: string, min?: string, until?: string): boolean;
versionCheck(version: string, min?: Version, until?: Version): true;

// Constants
MINIMUM_VERSION = "2.1.0";
MAXIMUM_VERSION = "4.0.0";
defaultVersionCheckTimeout = 5000;
supportedSurrealDbVersionMin: Version;
supportedSurrealDbVersionUntil: Version;
supportedSurrealDbVersionRange: string;
```

### `Features`

```ts
const Features: Readonly<{
  LiveQueries: Feature;
  Sessions: Feature;
  Api: Feature;
  RefreshTokens: Feature;
  Transactions: Feature;
  ExportImportRaw: Feature;
  SurrealML: Feature;
}>;
```

### `ChannelIterator<T>`

```ts
class ChannelIterator<T> implements AsyncIterable<T>, AsyncIterator<T> {
  constructor(cleanup?: () => void);
  next(): Promise<IteratorResult<T>>;
  return(): Promise<IteratorResult<T>>;
  throw(error?: unknown): Promise<IteratorResult<T>>;
  [Symbol.asyncIterator](): this;
  submit(value: T): void;
  cancel(): void;
}
```

### `Emit` / `Publisher`

```ts
class Publisher<T extends EventPayload> implements EventPublisher<T> {
  subscribe<K extends keyof T>(event: K, listener: (...event: T[K]) => void): () => void;
  subscribeFirst<K extends keyof T>(...events: K[]): Promise<T[K]>;
  publish<K extends keyof T>(event: K, ...payload: T[K]): void;
}
```

### `getIncrementalID()`

```ts
getIncrementalID(): string;
```

---

## 9. Expression Builder

### `expr()`

```ts
expr(expr: ExprLike): BoundQuery;
```

### Comparison Operators

```ts
raw(s: string)                    // Raw SurrealQL (WARNING: injection risk)
eq(field: string, v: unknown)     // =
eeq(field: string, v: unknown)    // ==
ne(field: string, v: unknown)     // !=
gt(field: string, v: unknown)     // >
gte(field: string, v: unknown)    // >=
lt(field: string, v: unknown)     // <
lte(field: string, v: unknown)    // <=
```

### Array / Contains Operators

```ts
contains(field: string, v: unknown)       // CONTAINS
containsAny(field: string, v: unknown)    // CONTAINSANY
containsAll(field: string, v: unknown)    // CONTAINSALL
containsNone(field: string, v: unknown)   // CONTAINSNONE
inside(field: string, v: unknown)         // INSIDE
```

### Geometry Operators

```ts
outside(field: string, g: unknown)        // OUTSIDE
intersects(field: string, g: unknown)     // INTERSECTS
```

### Text Search

```ts
matches(field: string, q: string, ref?: number);  // @@ full-text search
```

### Vector / KNN

```ts
knn(field: string, v: unknown, neighbors: number, metricOrEf?: string | number);
// Brute Force: <|n,metric|>
// MTree: <|n|>
// HNSW: <|n,ef|>
```

### Range / Logic

```ts
between(field: string, a: unknown, b: unknown)    // Shortcut for and(gte, lte)
and(...exprs: ExprLike[]): Expr
or(...exprs: ExprLike[]): Expr
not(expr: ExprLike): Expr
```

### Interfaces

```ts
interface Expr {
  toSQL(ctx: ExprCtx): string;
}
type ExprLike = Expr | null | undefined | false;
interface ExprCtx {
  def: (value: unknown) => string;
}
```

---

## 10. TypeScript Types

### Connection Types

```ts
type ConnectionStatus = "disconnected" | "connecting" | "reconnecting" | "connected";

interface DriverOptions {
  engines?: Engines;
  codecs?: Codecs;
  codecOptions?: CodecOptions;
  websocketImpl?: typeof WebSocket;
  fetchImpl?: typeof fetch;
}

interface ConnectOptions {
  namespace?: string;
  database?: string;
  authentication?: AuthProvider;
  versionCheck?: boolean;       // default: true
  invalidateOnExpiry?: boolean; // default: false
  reconnect?: boolean | Partial<ReconnectOptions>; // default: true
}

interface ReconnectOptions {
  enabled: boolean;
  attempts: number;               // -1 = unlimited
  retryDelay: number;
  retryDelayMax: number;
  retryDelayMultiplier: number;
  retryDelayJitter: number;       // float 0-1
  catch?: (error: Error) => boolean;
}
```

### Codec Options

```ts
interface CodecOptions {
  useNativeDates?: boolean;
  valueEncodeVisitor?: (value: unknown) => unknown;
  valueDecodeVisitor?: (value: unknown) => unknown;
}

interface ValueCodec {
  encode: <T>(data: T) => Uint8Array;
  decode: <T>(data: Uint8Array) => T;
}

type CodecType = "cbor" | "flatbuffer" | (string & {});
type Codecs = Partial<Record<CodecType, CodecFactory>>;
type CodecRegistry = Record<CodecType, ValueCodec>;
```

### Query Result Types

```ts
type RecordResult<T> = Prettify<T extends { id: infer Id } ? ... : { id: RecordId } & T>;

interface QueryStats {
  recordsReceived: number;
  bytesReceived: number;
  recordsScanned: number;
  bytesScanned: number;
  duration: Duration;
}

interface QueryChunk<T> {
  query: number;
  batch: number;
  kind: QueryResponseKind;   // "single" | "batched" | "batched-final"
  stats?: QueryStats;
  result?: T[];
  type?: QueryType;          // "live" | "kill" | "other"
  error?: ServerError;
}

type QueryResponse<T = unknown> = QueryResponseSuccess<T> | QueryResponseFailure;
type QueryResponseSuccess<T> = { success: true; stats?: QueryStats; type: QueryType; result: T };
type QueryResponseFailure = { success: false; stats?: QueryStats; error: ServerError };

// Legacy RPC query result types
type RpcQueryResult<T> = RpcQueryResultOk<T> | RpcQueryResultErr;
type RpcQueryResultOk<T> = { status: "OK"; time: string; result: T; type: QueryType };
type RpcQueryResultErr = { status: "ERR"; time: string; result: string; kind?: string; details?: ... };
```

### Patch Types (JSON Patch)

```ts
type Patch =
  | AddPatch      // { op: "add"; path: string; value: unknown }
  | RemovePatch   // { op: "remove"; path: string }
  | ReplacePatch  // { op: "replace"; path: string; value: unknown }
  | ChangePatch   // { op: "change"; path: string; value: string }
  | CopyPatch     // { op: "copy"; path: string; from: string }
  | MovePatch     // { op: "move"; path: string; from: string }
  | TestPatch     // { op: "test"; path: string; value: unknown }
```

### Utility Types

```ts
type Prettify<T> = { [K in keyof T]: T[K] } & {};
type Values<T> = Partial<T> & Record<string, unknown>;
type Output = "none" | "null" | "diff" | "before" | "after";
type Mutation = "content" | "merge" | "replace" | "patch";
type Nullable<T> = { [K in keyof T]: T[K] | null };
type Session = Uuid | undefined;
type Field<I> = keyof I | (string & {});
type Selection = "value" | "fields" | "diff";

type Jsonify<T> = /* recursive JSON-safe conversion */;
type MaybeJsonify<T, J extends boolean> = J extends true ? Jsonify<T> : T;
type RecordIdValue = string | number | Uuid | bigint | unknown[] | Record<string, unknown>;
type AnyRecordId = RecordId | StringRecordId;
type LiveResource = Table;

type Version = `${number}.${number}.${number}`;
type DataStream = string | ReadableStream;
type QueryType = "live" | "kill" | "other";
type QueryResponseKind = "single" | "batched" | "batched-final";
```

### Live Query Types

```ts
const LIVE_ACTIONS: readonly ["CREATE", "UPDATE", "DELETE", "KILLED"];
type LiveAction = "CREATE" | "UPDATE" | "DELETE" | "KILLED";

interface LiveMessage {
  queryId: Uuid;
  action: LiveAction;
  recordId: RecordId;
  value: Record<string, unknown>;
}

type LiveHandlerArguments<Result> =
  | [action: LiveAction, result: Result]
  | [action: "CLOSE", result: "killed" | "disconnected"];

type LiveHandler<Result> = (...[action, result]: LiveHandlerArguments<Result>) => unknown;
```

### Namespace / Database

```ts
interface NamespaceDatabase {
  namespace?: string;
  database?: string;
}
```

### Export Options

```ts
interface SqlExportOptions {
  users: boolean;
  accesses: boolean;
  params: boolean;
  functions: boolean;
  analyzers: boolean;
  apis: boolean;
  buckets: boolean;
  modules: boolean;
  configs: boolean;
  tables: boolean | string[];
  versions: boolean;
  records: boolean;
  sequences: boolean;
  v3: boolean;
}

interface MlExportOptions {
  name: string;
  version: string;
}
```

### SurrealApi / User-Defined APIs

```ts
type DefaultPaths = { [path: string]: PathDef };
type PathDef = Partial<Record<HttpMethod, MethodDef>>;
type HttpMethod = "get" | "post" | "put" | "delete" | "patch" | "trace";
```

---

## 11. Error Classes

### Base Errors

```ts
class SurrealError extends Error {}  // Base class for all SDK errors

// Connection errors
class CallTerminatedError extends SurrealError {}        // Call closed due to disconnect
class ConnectionUnavailableError extends SurrealError {}  // No connection available
class MissingNamespaceDatabaseError extends SurrealError {} // No ns/db selected
class HttpConnectionError extends SurrealError {          // HTTP connection failure
  readonly status: number;
  readonly statusText: string;
  readonly buffer: ArrayBuffer;
}
class ReconnectExhaustionError extends SurrealError {}     // Reconnect attempts exhausted
class ReconnectIterationError extends SurrealError {}      // Reconnect iteration failed

// Server errors
class ServerError extends SurrealError {                   // Base for all server errors
  readonly kind: string;                                   // Error category
  readonly code: number;                                   // Legacy RPC code
  readonly details: ErrorDetail | undefined;               // Structured details
  readonly cause: ServerError | undefined;                 // Nested cause chain
}
class ValidationError extends ServerError {                // kind: "Validation"
  get isParseError(): boolean;
  get parameterName(): string | undefined;
}
class ConfigurationError extends ServerError {              // kind: "Configuration"
  get isLiveQueryNotSupported(): boolean;
}
class ThrownError extends ServerError {}                    // kind: "Thrown" (THROW in SurrealQL)
class QueryError extends ServerError {                     // kind: "Query"
  get isNotExecuted(): boolean;
  get isTimedOut(): boolean;
  get isCancelled(): boolean;
  get timeout(): { secs: number; nanos: number } | undefined;
}
class SerializationError extends ServerError {             // kind: "Serialization"
  get isDeserialization(): boolean;
}
class NotAllowedError extends ServerError {                // kind: "NotAllowed"
  get isTokenExpired(): boolean;
  get isInvalidAuth(): boolean;
  get isScriptingBlocked(): boolean;
  get methodName(): string | undefined;
  get functionName(): string | undefined;
}
class NotFoundError extends ServerError {                  // kind: "NotFound"
  get tableName(): string | undefined;
  get recordId(): string | undefined;
  get methodName(): string | undefined;
  get namespaceName(): string | undefined;
  get databaseName(): string | undefined;
}
class AlreadyExistsError extends ServerError {              // kind: "AlreadyExists"
  get recordId(): string | undefined;
  get tableName(): string | undefined;
}
class InternalError extends ServerError {}                  // kind: "Internal"

// Auth errors
class AuthenticationError extends SurrealError {}
class LiveSubscriptionError extends SurrealError {}
class UnsupportedVersionError extends SurrealError {       // Version not in supported range
  readonly version: string;
  readonly minimum: string;
  readonly maximum: string;
}

// Value errors
class ExpressionError extends SurrealError {}
class InvalidDateError extends SurrealError {}
class InvalidRecordIdError extends SurrealError {}
class InvalidDurationError extends SurrealError {}
class InvalidDecimalError extends SurrealError {}
class InvalidTableError extends SurrealError {}

// Feature errors
class UnsupportedFeatureError extends SurrealError {
  readonly feature: Feature;
}
class UnavailableFeatureError extends SurrealError {
  readonly feature: Feature;
  readonly version: string;
}

// Other errors
class PublishError extends SurrealError { readonly causes: unknown[] }
class InvalidSessionError extends SurrealError { readonly session: Session }
class UnsuccessfulApiError extends SurrealError { readonly path: string; readonly method: string; readonly response: ApiResponse<unknown> }
class UnexpectedServerResponseError extends SurrealError { readonly response: unknown }
class UnexpectedConnectionError extends SurrealError { constructor(cause: unknown) }
class UnsupportedEngineError extends SurrealError { readonly engine: string }
```

### Server Error Detail Types

```ts
const ErrorKind = {
  Validation: "Validation",
  Configuration: "Configuration",
  Thrown: "Thrown",
  Query: "Query",
  Serialization: "Serialization",
  NotAllowed: "NotAllowed",
  NotFound: "NotFound",
  AlreadyExists: "AlreadyExists",
  Connection: "Connection",
  Internal: "Internal",
};

// Structured detail types:
type AuthErrorDetail = /* TokenExpired | SessionExpired | InvalidAuth | etc. */
type ValidationErrorDetail = /* Parse | InvalidRequest | InvalidParams | etc. */
type ConfigurationErrorDetail = /* LiveQueryNotSupported | BadLiveQueryConfig | BadGraphqlConfig */
type QueryErrorDetail = /* NotExecuted | TimedOut | Cancelled */
type SerializationErrorDetail = /* Serialization | Deserialization */
type NotAllowedErrorDetail = /* Scripting | Auth | Method | Function | Target */
type NotFoundErrorDetail = /* Method | Session | Table | Record | Namespace | Database | Transaction */
type AlreadyExistsErrorDetail = /* Session | Table | Record | Namespace | Database */
type ConnectionErrorDetail = /* Uninitialised | AlreadyConnected */
```

### Error Parsing

```ts
parseRpcError(raw: RpcErrorObject): ServerError;
```

---

## 12. Live Queries & Subscriptions

### `ManagedLiveSubscription`

```ts
class ManagedLiveSubscription extends LiveSubscription {
  get id(): Uuid;
  get isManaged(): boolean;
  get resource(): LiveResource;
  get isAlive(): boolean;
  kill(): Promise<void>;
  [Symbol.asyncIterator](): AsyncIterator<LiveMessage>;
}
```

### `UnmanagedLiveSubscription`

```ts
class UnmanagedLiveSubscription extends LiveSubscription {
  get id(): Uuid;
  get isManaged(): boolean;
  get resource(): undefined;
  get isAlive(): boolean;
  kill(): Promise<void>;
  [Symbol.asyncIterator](): AsyncIterator<LiveMessage>;
}
```

### Abstract Base

```ts
abstract class LiveSubscription implements AsyncIterable<LiveMessage> {
  abstract get id(): Uuid;
  abstract get isManaged(): boolean;
  abstract get resource(): LiveResource | undefined;
  abstract get isAlive(): boolean;
  abstract kill(): Promise<void>;
  abstract [Symbol.asyncIterator](): AsyncIterator<LiveMessage>;
  subscribe(handler: (message: LiveMessage) => void): () => void;
}
```

### Usage

```ts
// Subscribe to live query
const subscription = await db.live<Person>(new Table("person"));

// Iterate with for-await
for await (const message of subscription) {
  console.log(message.action, message.recordId, message.value);
}

// Or use callback
subscription.subscribe(({ action, recordId, value }) => {
  if (action === "CREATE") console.log("New record:", value);
});

// Kill when done
await subscription.kill();

// Subscribe to existing (unmanaged) live query
const existing = await db.liveOf(uuid);
```

---

## 13. Engine & Codec Interfaces

### `SurrealProtocol`

```ts
interface SurrealProtocol {
  health(): Promise<void>;
  version(): Promise<VersionInfo>;
  sessions(): Promise<Uuid[]>;
  attach(session: Uuid): Promise<void>;
  detach(session: Uuid): Promise<void>;
  use(what: Nullable<NamespaceDatabase>, session: Session): Promise<NamespaceDatabase>;
  signup(auth: AccessRecordAuth, session: Session): Promise<Tokens>;
  signin(auth: AnyAuth, session: Session): Promise<Tokens>;
  authenticate(token: Token, session: Session): Promise<void>;
  set(name: string, value: unknown, session: Session): Promise<void>;
  unset(name: string, session: Session): Promise<void>;
  refresh(tokens: Tokens, session: Session): Promise<Tokens>;
  revoke(tokens: Tokens, session: Session): Promise<void>;
  invalidate(session: Session): Promise<void>;
  reset(session: Session): Promise<void>;
  begin(session: Session): Promise<Uuid>;
  commit(txn: Uuid, session: Session): Promise<void>;
  cancel(txn: Uuid, session: Session): Promise<void>;
  importSql(data: string | Blob | ReadableStream): Promise<void>;
  exportSql(options: Partial<SqlExportOptions>): Promise<Response | string>;
  exportMlModel(options: MlExportOptions): Promise<Response | Uint8Array>;
  query<T>(query: BoundQuery, session: Session, txn?: Uuid): AsyncIterable<QueryChunk<T>>;
  liveQuery(id: Uuid): AsyncIterable<LiveMessage>;
}
```

### `SurrealEngine`

```ts
interface SurrealEngine extends SurrealProtocol, EventPublisher<EngineEvents> {
  features: Set<Feature>;
  open(state: ConnectionState): void;
  close(): Promise<void>;
  ready(): void;
}
```

### Built-in Engines

```ts
abstract class RpcEngine implements SurrealProtocol { /* base for JSON-based engines */ }
class HttpEngine extends RpcEngine implements SurrealEngine { /* HTTP stateless */ }
class WebSocketEngine extends RpcEngine implements SurrealEngine { /* WebSocket stateful */ }

// Factory
createRemoteEngines(): Engines;
```

### `CborCodec`

```ts
class CborCodec implements ValueCodec {
  constructor(options: CodecOptions);
  encode<T>(data: T): Uint8Array;
  decode<T>(data: Uint8Array): T;
}
```

---

## 14. Diagnostics

```ts
type DiagnosticKey = keyof DiagnosticMap;

type DiagnosticEvent<T extends DiagnosticKey> =
  | { type: T; key: Uuid; phase: "before" }
  | { type: T; key: Uuid; phase: "progress"; result: DiagnosticResult<T> }
  | { type: T; key: Uuid; phase: "after"; duration: Duration; success: false; error: Error }
  | { type: T; key: Uuid; phase: "after"; duration: Duration; success: true; result: DiagnosticResult<T> };

type Diagnostic = { [K in DiagnosticMap]: DiagnosticEvent<K> }[DiagnosticMap];

applyDiagnostics(engines: Engines, callback: (event: Diagnostic) => void): Engines;
```

### Diagnostic Map

```ts
type DiagnosticMap = {
  query:       QueryInfo & SessionInfo;
  liveQuery:   LiveQueryInfo;
  version:     VersionInfo;
  signup:      AuthInfo & SessionInfo;
  signin:      AuthInfo & SessionInfo;
  authenticate: AuthInfo & SessionInfo;
  open:        OpenInfo;
  close:       undefined;
  health:      undefined;
  use:         UseInfo & SessionInfo;
  set:         SetInfo & SessionInfo;
  unset:       UnsetInfo & SessionInfo;
  refresh:     SessionInfo;
  revoke:      SessionInfo;
  invalidate:  SessionInfo;
  reset:       SessionInfo;
  begin:       SessionInfo;
  commit:      TransactionInfo & SessionInfo;
  cancel:      TransactionInfo & SessionInfo;
  sessions:    Uuid[];
  attach:      undefined;
  detach:      undefined;
  importSql:   undefined;
  exportSql:   undefined;
  exportMlModel: undefined;
};
```

---

## Reference: All Exports

### From `surrealdb`

**Classes:** `Surreal`, `SurrealSession`, `SurrealTransaction`, `SurrealQueryable`, `SurrealApi`

**Value Types:** `RecordId`, `RecordIdRange`, `StringRecordId`, `Table`, `Uuid`, `DateTime`, `Duration`, `Decimal`, `FileRef`, `Future`, `Range`, `Geometry`, `GeometryPoint`, `GeometryLine`, `GeometryPolygon`, `GeometryMultiPoint`, `GeometryMultiLine`, `GeometryMultiPolygon`, `GeometryCollection`, `BoundIncluded`, `BoundExcluded`, `Value`

**Query Builders:** `SelectPromise`, `CreatePromise`, `UpdatePromise`, `UpsertPromise`, `InsertPromise`, `DeletePromise`, `RelatePromise`, `Query`, `RunPromise`, `AuthPromise`, `ApiPromise`, `ManagedLivePromise`, `UnmanagedLivePromise`, `ExportPromise`, `ExportModelPromise`

**Live/Frame:** `LiveSubscription`, `ManagedLiveSubscription`, `UnmanagedLiveSubscription`, `Frame`, `ValueFrame`, `ErrorFrame`, `DoneFrame`

**Errors:** `SurrealError`, `ServerError`, `ValidationError`, `ConfigurationError`, `ThrownError`, `QueryError`, `SerializationError`, `NotAllowedError`, `NotFoundError`, `AlreadyExistsError`, `InternalError`, `AuthenticationError`, `CallTerminatedError`, `ConnectionUnavailableError`, `MissingNamespaceDatabaseError`, `HttpConnectionError`, `ReconnectExhaustionError`, `ReconnectIterationError`, `LiveSubscriptionError`, `UnsupportedVersionError`, `ExpressionError`, `InvalidDateError`, `InvalidRecordIdError`, `InvalidDurationError`, `InvalidDecimalError`, `InvalidTableError`, `UnsupportedFeatureError`, `UnavailableFeatureError`, `PublishError`, `InvalidSessionError`, `UnsuccessfulApiError`, `UnexpectedServerResponseError`, `UnexpectedConnectionError`, `UnsupportedEngineError`

**Utilities:** `surql`, `expr`, `raw`, `eq`, `eeq`, `ne`, `gt`, `gte`, `lt`, `lte`, `contains`, `containsAny`, `containsAll`, `containsNone`, `inside`, `outside`, `intersects`, `matches`, `knn`, `between`, `and`, `or`, `not`, `s`, `d`, `r`, `u`, `toSurqlString`, `jsonify`, `equals`, `escapeIdent`, `escapeNumber`, `escapeIdPart`, `escapeRangeBound`, `mergeBindings`, `BoundQuery`, `ChannelIterator`, `getIncrementalID`, `isLiveResult`, `isVersionSupported`, `versionCheck`, `retrieveRemoteVersion`, `parseRpcError`

**Constants/Types:** `Features`, `LIVE_ACTIONS`, `ErrorKind`, `MINIMUM_VERSION`, `MAXIMUM_VERSION`, `defaultVersionCheckTimeout`, `supportedSurrealDbVersionMin`, `supportedSurrealDbVersionUntil`, `supportedSurrealDbVersionRange`, `DEFAULT_RECONNECT_OPTIONS`

**Interfaces/Types:** `ConnectOptions`, `ReconnectOptions`, `DriverOptions`, `CodecOptions`, `ValueCodec`, `ConnectionStatus`, `SurrealProtocol`, `SurrealEngine`, `EngineEvents`, `ConnectionState`, `ConnectionSession`, `DriverContext`, `SessionEvents`, `SurrealEvents`, `Expr`, `ExprCtx`, `ExprLike`, `Patch`, `QueryStats`, `QueryChunk`, `QueryResponse`, `QueryResponseSuccess`, `QueryResponseFailure`, `NamesRecordsult`, `RpcRequest`, `RpcResponse`, `RpcErrorObject`, `RpcErrorCause`

**Auth Types:** `RootAuth`, `NamespaceAuth`, `DatabaseAuth`, `AccessSystemAuth`, `AccessBearerAuth`, `AccessRecordAuth`, `SystemAuth`, `AccessAuth`, `AnyAuth`, `Token`, `Tokens`, `AuthProvider`, `AuthCallable`, `ProvidedAuth`, `AuthOrToken`

**Engine/Factories:** `createRemoteEngines`, `applyDiagnostics`, `HttpEngine`, `WebSocketEngine`, `RpcEngine`, `CborCodec`, `EngineFactory`, `Engines`, `CodecFactory`, `Codecs`, `CodecRegistry`

**Utility Types:** `Prettify`, `Values`, `Output`, `Mutation`, `Nullable`, `Session`, `Field`, `RecordIdValue`, `AnyRecordId`, `LiveResource`, `LiveAction`, `LiveMessage`, `Jsonify`, `MapQueryResult`, `Version`, `RecordResult`, `NamespaceDatabase`, `SqlExportOptions`, `MlExportOptions`, `Diagnostic`, `DiagnosticKey`, `DiagnosticResult`, `DiagnosticEvent`

