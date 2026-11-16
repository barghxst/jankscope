# 002. Модель кадров и выбор потока рендерера

**Статус:** не начато
**Крейты:** `crates/core` (модуль `frames`), `crates/cli` (команда `frames`)
**Оценка:** 3–4 вечера

## Цель

Найти в трейсе процесс страницы и его потоки, собрать из событий список кадров и показать,
какие кадры пользователь увидел, а какие потерялись:

```
$ jankscope frames fixtures/recorded/layout-thrashing.trace.json
renderer: pid 46120 · main 1 · compositor 9
vsync:    16.7 ms
frames:   51 · presented 40 · dropped 10 · idle 1
longest stall: 167.3 ms (frames 194–203)
```

Это первый настоящий анализ: по этим числам уже видно, что клик на тестовой странице заморозил её на 10 кадров
подряд. В спеке 003 мы ответим на вопрос «почему» (forced layout), а здесь отвечаем на вопрос «что и когда».

## Чему научишься

**Rust**
- Второй модуль в крейте и связь между модулями: `use crate::trace::...`.
- Времена жизни: почему метод, который возвращает ссылки на события, пишется как `-> impl Iterator<Item = &TraceEvent> + '_`.
- `Option` внутри функции: `?` для `Option`, `and_then`, `as_u64()` у `serde_json::Value`.
- Enum без данных и `#[derive(Clone, Copy)]`: чем `Copy` отличается от `Clone`.
- Ошибка с данными внутри варианта: `Ambiguous(Vec<i64>)`.
- `HashMap<u64, f64>` как индекс «номер кадра → время отрисовки».
- Сортировка `f64`: `sort_by(f64::total_cmp)` (продолжение темы «у `f64` нет `Ord`»).
- Срезы: `windows(2)` для соседних пар, `chunk_by` для серий подряд идущих элементов.
- `impl fmt::Display` для своего типа.

**Браузер**
- Как кадр проходит через Chrome: vsync → compositor → main thread → commit → draw.
- Почему compositor продолжает тикать, когда главный поток занят, и что значит «потерянный кадр».
- Как DevTools понимает, какой процесс отвечает за страницу.

## Немного теории: путь одного кадра

У каждого процесса рендерера два главных потока:

| Поток | Имя в трейсе | Что делает |
|---|---|---|
| main | `CrRendererMain` | JS, стили, layout, paint. Всё, что пишет страница |
| compositor | `Compositor` | собирает слои в кадр и отдаёт его на экран. JS сюда не попадает |

Экран обновляется 60 раз в секунду, то есть раз в 16.7 мс (это **vsync**). Каждый такт идёт так:

```
compositor:  BeginFrame(seq) ─► RequestMainThreadFrame
main:                             BeginMainThreadFrame ─► rAF, стили, layout, paint ─► Commit
compositor:                                                                     ActivateLayerTree ─► DrawFrame(seq)
```

Если всё уложилось в такт, у `BeginFrame` и `DrawFrame` одинаковый `frameSeqId`, и кадр **показан** (presented).
Если главный поток занят, compositor всё равно получает следующие `BeginFrame`, но рисовать ему нечего:
новых данных от main нет. Такие такты без `DrawFrame` и есть **потерянные кадры** (dropped).

Реальные события из нашего трейса (compositor, `pid 46120`, `tid 9`):

```json
{"name":"BeginFrame","ph":"I","s":"t","pid":46120,"tid":9,"ts":1740947026,
 "cat":"disabled-by-default-devtools.timeline.frame","args":{"frameSeqId":204,"layerTreeId":2}}
{"name":"DrawFrame","ph":"I","s":"t","pid":46120,"tid":9,"ts":1740947705,
 "cat":"disabled-by-default-devtools.timeline.frame","args":{"frameSeqId":204,"layerTreeId":2}}
```

Что случилось на тестовой странице после клика, по времени (в мс от `BeginFrame 194`):

| Время | Поток | Событие |
|---|---|---|
| 0.0 | compositor | `BeginFrame 194` |
| 11.2 | main | `EventDispatch click`, длится **152.6 мс** (`badHandler`: 400 раз «записать стиль → прочитать размер») |
| 16.7 … 150.0 | compositor | `BeginFrame 195` … `203`: такты идут, `DrawFrame` нет |
| 166.7 | compositor | `BeginFrame 204` |
| 167.3 | compositor | `DrawFrame 204`, первый кадр после клика |
| 183.3 … 817.0 | compositor | `BeginFrame` и `DrawFrame` 205 … 243 парами: анимация `left` идёт ровно |
| 833.3 | compositor | `BeginFrame 244`, `DrawFrame` нет: анимация кончилась, рисовать нечего |

Итог: 51 такт (`194`–`244`), из них 40 с `DrawFrame`, 10 потерянных (`194`–`203`) и 1 холостой (`244`).
Пауза для пользователя: от `BeginFrame 194` до `DrawFrame 204`, то есть 167.3 мс.

### Подвохи, найденные в реальном трейсе

1. **`tid` не уникален.** В процессе рендерера потоки пронумерованы с единицы (`46120:1`, `46120:9`), потому что он живёт
   в своём sandbox. В другом рендерере тоже будет `tid 1`. Поток определяет только пара `(pid, tid)`.
2. **Рендереров бывает несколько.** На реальном сайте у `about:blank`, расширений и iframe из другого домена свои
   процессы со своим `CrRendererMain`. В нашем трейсе он один, но выбирать «первый попавшийся» нельзя.
3. **Какой процесс главный, знает браузер.** Событие `TracingStartedInBrowser` в потоке `CrBrowserMain` перечисляет
   фреймы страницы:
   ```json
   {"name":"TracingStartedInBrowser","pid":46003,"tid":46003,"ph":"I",
    "args":{"data":{"frames":[{"frame":"C761BB02...","isOutermostMainFrame":true,"processId":46120,
                               "url":"file:///.../layout-thrashing.html"}]}}}
   ```
   Нам нужен `processId` фрейма с `isOutermostMainFrame: true`.
4. **Два разных номера кадра.** `BeginFrame` и `DrawFrame` несут `args.frameSeqId` (194, 195, …), а
   `BeginMainThreadFrame` несёт `args.data.frameId` (2, 3, …). Это **разные** счётчики, сравнивать их нельзя.
   Мы опираемся только на `frameSeqId`.
5. **Одинаковые имена в разных потоках.** `Commit` есть и на main (`ph: X`), и на compositor (`ph: b/e`, без `frameSeqId`).
   Поэтому события сначала фильтруем по `(pid, tid)` и только потом по имени.
6. **`BeginFrame` без `DrawFrame` бывает не только потерянным.** Последний такт `244` пришёл, когда анимация уже
   закончилась, и сразу после него compositor отписался от vsync (`NeedsBeginFrameChanged`, `needsBeginFrame: 0`).
   Это холостой такт (idle), не jank. Правило для этой спеки: такт без `DrawFrame` считается потерянным, только если
   **позже** был показанный кадр. Хвост без показанных кадров считается холостым.
7. **Поле `s: "t"`** у мгновенных событий означает scope «поток». Нам оно не нужно, в `TraceEvent` не добавляем.

## Что сделать

После каждого шага `cargo test` зелёный. Все проверки ниже можно сверить через `jq` на большом трейсе.

### Шаг 1. Модуль и доступ к `args`

- Создать `crates/core/src/frames.rs`, объявить в `lib.rs`.
- Добавить на `TraceEvent` метод, который достаёт номер кадра:
  ```rust
  impl TraceEvent {
      pub fn frame_seq_id(&self) -> Option<u64>;  // args.frameSeqId
  }
  ```
- Тесты: у `BeginFrame` из примера выше получается `Some(204)`; у события без `args` получается `None`;
  у `"frameSeqId": "204"` (строка) тоже `None`, без паники.

> **Разберёмся:** `?` в функции, которая возвращает `Option`, `Value::get` и `as_u64`, почему здесь не нужен `match`.

### Шаг 2. Выбор рендерера

```rust
pub struct Renderer { pub pid: i64, pub main_tid: i64, pub compositor_tid: i64 }

#[derive(Debug, Error)]
pub enum RendererError {
    #[error("no renderer main thread in trace")]
    NotFound,
    #[error("several renderers and no TracingStartedInBrowser: {0:?}")]
    Ambiguous(Vec<i64>),
    #[error("renderer {0} has no compositor thread")]
    NoCompositor(i64),
}

impl Trace {
    pub fn main_renderer(&self) -> Result<Renderer, RendererError>;
}
```

Алгоритм:
1. Если есть `TracingStartedInBrowser`, взять `processId` фрейма с `isOutermostMainFrame: true`.
2. Иначе собрать все `pid`, у которых есть поток `CrRendererMain`. Один `pid` → он. Несколько → `Ambiguous`. Ноль → `NotFound`.
3. В выбранном процессе найти `tid` потоков `CrRendererMain` и `Compositor` через `thread_names()` из спеки 001.

Тесты: на большом трейсе `pid 46120 · main 1 · compositor 9`. Unit-тесты на маленьком JSON прямо в файле:
два рендерера без `TracingStartedInBrowser` дают `Ambiguous(vec![...])`, два рендерера с ним дают нужный.

> **Разберёмся:** enum ошибки с данными, `{0:?}` в `thiserror`, цепочка `and_then` по вложенному JSON,
> `iter().find(...)`.

### Шаг 3. События одного потока

```rust
impl Trace {
    pub fn events_on(&self, pid: i64, tid: i64) -> impl Iterator<Item = &TraceEvent> + '_;
}
```

Проверка: на большом трейсе на `46120:9` ровно 51 `BeginFrame` и 40 `DrawFrame`.

> **Разберёмся:** ленивые итераторы (ничего не считается, пока не вызван `collect` или `count`), что такое `'_`
> и почему без него компилятор ругается. Сравним с генераторами в JS.

### Шаг 4. Кадры

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameStatus { Presented, Dropped, Idle }

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub seq: u64,
    pub begin_ts: f64,
    pub draw_ts: Option<f64>,
    pub status: FrameStatus,
}

pub fn frames(trace: &Trace, renderer: &Renderer) -> Vec<Frame>;
```

- Один `Frame` на каждый `BeginFrame` в потоке compositor, отсортированные по `seq`.
- `draw_ts` берётся из `DrawFrame` с тем же `frameSeqId`. Индекс `HashMap<u64, f64>`, а не поиск по списку на каждый кадр.
- Статус по правилу из подвоха 6.

Проверка на большом трейсе: 51 кадр, 40 `Presented`, 10 `Dropped` (`194`–`203`), 1 `Idle` (`244`).

> **Разберёмся:** `Copy` у `FrameStatus`; почему `Frame` тоже мог бы быть `Copy` (все поля `Copy`), а `TraceEvent` не может (`String`),
> два прохода по данным против одного, `HashMap::get` возвращает `Option<&f64>`, и откуда там `&`.

### Шаг 5. Такт vsync и паузы

```rust
pub struct FrameStats {
    pub vsync_ms: f64,
    pub presented: usize,
    pub dropped: usize,
    pub idle: usize,
    pub longest_stall: Option<Stall>,
}

pub struct Stall { pub first_seq: u64, pub last_seq: u64, pub duration_ms: f64 }

pub fn frame_stats(frames: &[Frame]) -> FrameStats;
```

- `vsync_ms`: медиана разниц между соседними `begin_ts`. В нашем трейсе все 50 разниц равны 16666 мкс.
  Медиана, а не среднее: один пропущенный `BeginFrame` (если compositor отписался и подписался обратно) не должен сдвигать такт.
- `Stall`: серия `Dropped` подряд. Длительность считается от `begin_ts` первого потерянного до `draw_ts` следующего
  показанного кадра, то есть ровно столько, сколько пользователь видел старую картинку.
- Самая длинная серия на большом трейсе: `194`–`203`, `1740947705 − 1740780366 = 167339` мкс, то есть 167.3 мс.

> **Разберёмся:** `windows(2)`, `chunk_by`, `sort_by(f64::total_cmp)`, `max_by(...)` для `f64`, почему функция принимает
> `&[Frame]`, а не `&Vec<Frame>`.

### Шаг 6. Команда `frames` в CLI

`jankscope frames <path>` печатает сводку как в разделе «Цель». Ошибка выбора рендерера печатается текстом
из `RendererError` и даёт ненулевой код выхода. Красивый вывод делаем через `impl fmt::Display for FrameStats`.

Заодно поправь хвост из спеки 001: в `crates/cli/src/main.rs` ошибка печатается через `eprint!`, без перевода строки.

> **Разберёмся:** `Display` против `Debug`, `{:.1}` для `f64`, как `match` по подкоманде растёт в `main` и почему
> пора задуматься о `clap` (но не сейчас).

### Шаг 7. Фикстура и интеграционные тесты

Фикстура `fixtures/traces/frames.trace.json` вырезается из большой записи. В ней метаданные, `TracingStartedInBrowser`
и все события тактов, всего 122 события:

```bash
jq '{traceEvents: [.traceEvents[] | select(.ph == "M" or .name == "TracingStartedInBrowser"
     or .name == "BeginFrame" or .name == "DrawFrame" or .name == "NeedsBeginFrameChanged")]}' \
  fixtures/recorded/layout-thrashing.trace.json > fixtures/traces/frames.trace.json
```

- Записать источник и команду в `fixtures/traces/README.md`.
- `crates/core/tests/frames.rs`: рендерер, число кадров по статусам, `vsync_ms`, самая длинная пауза. Числа те же,
  что на большом трейсе, потому что фикстура содержит все нужные события.

## Готово, когда

- [ ] `cargo test` зелёный: unit-тесты на подвохи 2, 4, 5 и 6, интеграционные на фикстуре `frames`.
- [ ] `cargo clippy` без предупреждений.
- [ ] `jankscope frames fixtures/recorded/layout-thrashing.trace.json` печатает сводку из раздела «Цель».
- [ ] Трейс без рендерера даёт понятную ошибку и ненулевой код выхода.
- [ ] В `core` по-прежнему нет `unwrap()` и `expect()` вне тестов.
- [ ] В `DECISIONS.md` записано решение: что считаем кадром (`BeginFrame` по `frameSeqId`) и правило dropped / idle.

## Что сознательно не делаем

- **Связь кадра с работой главного потока.** Какие задачи на main помешали кадру, будет в спеке 003.
- **`layerTreeId`.** В одном процессе бывает несколько деревьев слоёв (например, у iframe того же домена). Пока считаем
  все `BeginFrame` процесса; когда встретим трейс, где это мешает, отфильтруем по `layerTreeId` из `SetLayerTreeId`.
- **Частично показанные кадры и `PipelineReporter`.** Новые DevTools различают кадры с частичным обновлением по
  событиям `PipelineReporter` (в трейсе их 184). Это точнее, но сложнее. Наше правило проще и на тестовой странице
  даёт те же выводы.
- **Экраны 120 Гц.** `vsync_ms` вычисляется из трейса, а не захардкожен в 16.7, так что они поддерживаются сами.
  Но отдельно мы их не проверяем.
- **`clap`.**

## Что почитать

- [The Rust Book](https://doc.rust-lang.org/book/): глава 10.3 (времена жизни), 13.2 (итераторы), 7 (модули).
- [Life of a frame](https://chromium.googlesource.com/chromium/src/+/HEAD/docs/life_of_a_frame.md) в документации Chromium.
- [How cc works](https://chromium.googlesource.com/chromium/src/+/HEAD/docs/how_cc_works.md): раздел про scheduler и BeginFrame.
- DevTools frontend: `front_end/models/trace/handlers/FramesHandler.ts`, как DevTools решает ту же задачу.
