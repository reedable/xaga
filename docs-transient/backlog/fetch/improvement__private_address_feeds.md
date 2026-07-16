# Private/loopback feeds: unreachable, with no opt-in mechanism

The question this item is asking: **do we want to open a hole in the address policy to serve a use case, and is that use case worth the hole?**

- Hypothetical use case #1: a user subscribes to a feed hosted on their organization's private network (an intranet feed). Consequence: allowlisting that host/CIDR makes it reachable by any other feed's content too, not only the one the user meant to reach — a hostile feed could reference the same address.
- Hypothetical use case #2: a developer runs a feed on `localhost` during development and wants the app to fetch it. Consequence: a loopback-only opt-in reaches the same result with a smaller hole — LAN devices (routers, printers) stay unreachable, but any feed's content could still reference `localhost`/127.0.0.1 and reach whatever is listening there.

Facts known at time of writing:

- No setting today changes the unreachability; it applies to every consumer of the proxy.
- Any opt-in mechanism (allowlist, loopback-only, etc.) necessarily makes the allowlisted targets reachable by any feed content, not only the one the user had in mind — that's the hole being traded for the use case.

Options:

- **No mechanism (status quo).** The use case above, once established, stays unaddressed.
- **Opt-in allowlist of specific hosts or CIDRs** (analogous to `allowed_domains`), empty by default, disclosed to the user per `docs/principles/secure_by_default.md`.
- **Loopback-only opt-in**, a narrower hole than a full allowlist — LAN hosts (routers, printers) stay unreachable.

Whether any mechanism is wanted at all is the first decision, and it depends on establishing the use case and its value — scope and values follow from that.
