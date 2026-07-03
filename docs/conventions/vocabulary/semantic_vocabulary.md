# Semantic Vocabulary

Semantic vocabulary provides consistency to the names of symbols (variables and functions) used across the entire Xaga codebase. Variable and function names must remain honest about what they are, fit the developers' mental model at their level of abstraction. Unless explicit exception is warranted, they must not reflect their assumed usage or characteristics.

Where strict accuracy and the developer's mental model conflict, favor the mental model. A name that fits how a developer thinks is worth more than one that is technically precise but foreign. Pick the word at the developer's level of abstraction, not the most exact word available.

## Verbs

### Memory

- `get` Gets a value
- `set` Sets a value
- `add` Adds a value
- `remove` Removes a value
- `clear` Removes all values

### CRUD

- `list` Reads zero or more records.
- `read` Reads one record; existence usually known ahead of time.
- `find` Locates at most one record matching a criterion (0..1); existence not known ahead of time. By a singular state (an adjective): `find_<state>` (e.g. `find_default`). By a unique attribute: `find_<target>_by_<attribute>` (see Composition).
- `filter` A `list` narrowed by a matching attribute or state; 0..n matches. By state (an adjective): `filter_<state>` (e.g. `filter_unread`). By attribute: `filter_<members>_by_<attribute>` (see Composition).
- `search` Returns records ranked by relevance to a query; 0..n, ordered, not necessarily exhaustive. `query` is prohibited as a verb — use `find`/`filter`/`search`.
- `save` Creates or updates a record.
- `create` Creates a record.
- `delete` Deletes a record.
- `update` Updates a record.
- `upsert` Creates or updates a row (or rows), usually in a database.

#### Queue

CRUD over a collection. Separate policy decides which item `dequeue` returns next.

- `enqueue` `create`/`add` for a queue.
- `dequeue` Reads and deletes the next eligible item. Inverse of `enqueue`.

### IO

- `fetch` A network call; may be served from cache. Prefer `submit` for writes.
- `submit` A write (CUD) over the network.
- `list` Disc IO: gets a directory listing.
- `read` Disc IO: reads a single file.
- `write` Disc IO: writes content, overwriting if it exists.
- `update` Disc IO: updates a file only if it exists.

### Events

`on`/`off` are callback-based (push); `subscribe`/`unsubscribe` are stream-based (pull).

- `on` Registers a callback invoked on an event.
- `off` Deregisters a callback. Inverse of `on`.
- `dispatch` Dispatches an event.
- `subscribe` Subscribes to an event stream, returning a receiver.
- `unsubscribe` Ends a subscription. Inverse of `subscribe`.

### Predicates

- `is` Returns a boolean for a state or condition.
- `validate` Checks a value and returns the outcome (the error, or nothing). Not a boolean.

### Representation

`parse`/`format` and `deserialize`/`serialize` cross the same boundary (representation ↔ typed value); they differ by purpose and fidelity, not direction.

- `parse` Interprets an external representation (usually string) into an internal represetation. May be lossy; no round-trip guarantee.
- `format` Produces a external reprentation (usually string). The output is in the format preferred by the consumer, and may be human readable.
- `deserialize` Reconstructs a typed value from a machine encoding. Inverse of `serialize`.
- `serialize` Encodes a typed value into a machine encoding that round-trips losslessly. Inverse of `deserialize`.

#### Conversion / Mapping

Converts a typed value at one layer into a typed value at another. Both ends are typed values; if one end is a representation, use the verbs above instead. Named `<source>_to_<target>` (see Composition).

### Lifecycle

Each pair names a transition in a resource's life; not interchangeable.

- `open` Acquires a resource handle.
- `close` Releases a resource handle. Inverse of `open`; may be implicit via `Drop`.
- `start` Begins a long-lived running service.
- `shutdown` Gracefully ends a running service. Terminal.
- `abort` Abruptly ends a service or operation, dropping in-flight work. Terminal.
- `pause` Temporarily halts a service; non-terminal.
- `resume` Resumes a paused service. Inverse of `pause`.
- `connect` Establishes a connection.
- `disconnect` Tears down a connection. Inverse of `connect`.

Avoid `stop`: ambiguous between graceful (`shutdown`) and abrupt (`abort`).

## Nouns

- `value` A piece of data; the most abstract data noun, independent of storage or structure.
- `record` A structured `value`.
- `row` A `record` with a fixed column schema.
- `list` An ordered collection of records, accessed by position.
- `table` A collection of `row`s, accessed by column/key; not inherently ordered.
- `set` An unordered collection of unique `value`s, tested by membership.
- `queue` A collection items are added to and taken from under an ordering policy.
- `item` A single member of a collection.
- `element` An HTML/DOM node. Not a synonym for `item`; legitimate only where HTML is the subject (e.g. parsing).
- `id` A value that uniquely identifies a record.
- `payload` Data carried across a boundary (IPC, network).
- `request` A message asking for an operation to be performed. Preferred over `query`; use `query` only when searching, or for a specification carried within a request (e.g. a query string).
- `response` The reply to a `request`.
- `query` A specification of records to retrieve. May stand as a `request` when searching, or be a component of one (e.g. a query string).
- `result` The outcome of an operation.
- `error` A failure value describing why an operation failed.
- `batch` A group of items processed together as one unit.
- `sender` The channel endpoint that puts items in.
- `receiver` The channel endpoint that takes items out.
- `producer` The role that supplies items.
- `consumer` The role that takes and processes items.
- `resource` Something that must be acquired and released; accessed via a `handle`.
- `handle` A reference to an acquired resource; acquired by `open`, released by `close`.
- `service` A long-lived running process; managed by `start`/`shutdown`.
- `channel` A conduit that carries items from a `sender` to a `receiver`.
- `stream` A sequence of values delivered over time.
- `event` A notification that something happened.
- `callback` A function supplied to be invoked when an event occurs.
- `cache` A store of previously obtained values kept to avoid repeating an expensive operation.

`sender`/`receiver` are the channel endpoints; `producer`/`consumer` are the roles that use them.

Prohibited: `DTO`, `wire`. The boundary-role noun is `payload`.

### Relationships

A relationship connects two or more participants. Both participate in it.

- `connection` A link between two peers.
- `session` A stateful relationship between a client and a server across multiple interactions.
- `transaction` A bounded relationship between a caller and a resource, committed or rolled back as a unit.

## Composition

A symbol's name is composed of `Verbs` and `Nouns` joined by connectors.

- `to` Joins a source and target across a conversion: `<source>_to_<target>` (e.g. `md_to_entry`).
- `by` Names the attribute a thing is selected or keyed by. Unique attribute → `find_<target>_by_<attribute>` (0..1, e.g. `find_entry_by_url`); non-unique → `filter_<members>_by_<attribute>` (0..n, e.g. `filter_entries_by_author`); keyed collection → `<members>_by_<key>` (e.g. `entries_by_id`).
