// Curriculum content. Paths are relative to training/index.html.
const TRAINING = {
  "product": {
    "title": "Product owner / UX designer",
    "description": "Understand Xaga’s purpose, domain, workflows, and product trade-offs.",
    "lessons": [
      {
        "id": "purpose",
        "level": "Introduction",
        "title": "From discovery to insight",
        "objective": "Explain what Xaga is designed to help its user achieve.",
        "paragraphs": [
          "Xaga is a Personal Knowledge Management tool. Its stated purpose is to turn discovery into insight: discover material, understand it, connect it to existing knowledge, and arrive at an insight. Being first to receive an article is not the product goal.",
          "This purpose changes how to assess a proposal. Faster intake can support the workflow, but intake volume alone does not show that the user understands more. A useful product argument connects a proposed capability to reading, interpretation, connection, or revisiting.",
          "The README makes privacy and accessibility product commitments. It also draws a boundary: protecting the mechanics of fetching and storage does not establish whether an article is true. Discernment remains with the reader."
        ],
        "example": "A proposed “most articles opened today” badge measures activity. Before treating it as success, explain how it would help the reader understand or connect material.",
        "questions": [
          {
            "prompt": "Which outcome best matches Xaga’s stated purpose?",
            "answers": [
              {
                "text": "A reader connects an article to prior knowledge and develops an insight.",
                "feedback": "Understanding and connection are the purpose described in the README.",
                "correct": true
              },
              {
                "text": "The reader receives every article before anyone else.",
                "feedback": "Being first is explicitly outside the stated goal.",
                "correct": false
              },
              {
                "text": "The app guarantees that every saved article is true.",
                "feedback": "Xaga protects mechanics, not the truth of ideas.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A secure fetch succeeds. What can the product claim about the article?",
            "answers": [
              {
                "text": "It was fetched through the applicable controls; its claims still require discernment.",
                "feedback": "Transport and content credibility are separate questions.",
                "correct": true
              },
              {
                "text": "Its arguments have been fact-checked.",
                "feedback": "Network controls do not fact-check arguments.",
                "correct": false
              },
              {
                "text": "Its popularity proves its reliability.",
                "feedback": "The README explicitly warns against treating popularity as truth.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Product purpose",
            "path": "../../README.md"
          },
          {
            "title": "Secure by default",
            "path": "../principles/secure_by_default.md"
          }
        ]
      },
      {
        "id": "collections",
        "level": "Introduction",
        "title": "Feed, Topic, and Site",
        "objective": "Distinguish a managed collection from a derived browsing projection.",
        "paragraphs": [
          "Collection is the shared browsing abstraction for Feed, Topic, and Site. It supplies identity and counts. Feed and Topic are managed collections with their own records and lifecycle; “managed” does not mean that every member was selected manually.",
          "A Site is derived from the normalized hostname of stored article URLs. It has no independent membership file or creation timestamp. Feed URLs and feed membership do not decide Site membership. Articles arriving through unrelated feeds can belong to the same Site.",
          "The organization-model document describes the target domain and warns that some refactors may remain. For interface details, cross-check the specific behavior document and code rather than treating every domain possibility as a shipped feature."
        ],
        "example": "An article at https://www.example.com/story and another at https://example.com/other belong to example.com. An article at https://news.example.com/story belongs to a different Site.",
        "questions": [
          {
            "prompt": "Two unrelated feeds contain articles from example.com. How is their Site determined?",
            "answers": [
              {
                "text": "By each stored article URL’s normalized hostname.",
                "feedback": "The Site projection groups Entry URLs independently of feeds.",
                "correct": true
              },
              {
                "text": "By the feed that was added first.",
                "feedback": "Feed membership does not establish Site identity.",
                "correct": false
              },
              {
                "text": "By a manually maintained Site membership list.",
                "feedback": "Site membership is derived rather than separately persisted.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "What should a Site design avoid inventing?",
            "answers": [
              {
                "text": "A creation timestamp solely to satisfy a managed-collection shape.",
                "feedback": "A Site has no independent managed lifecycle.",
                "correct": true
              },
              {
                "text": "A normalized hostname used as identity.",
                "feedback": "That is the defined Site identity.",
                "correct": false
              },
              {
                "text": "Total and unread counts derived from Entries.",
                "feedback": "Those are part of the shared collection contract.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Organization model",
            "path": "../architecture/organization_model.md"
          },
          {
            "title": "Site identity and membership",
            "path": "../architecture/sites.md"
          },
          {
            "title": "Domain types",
            "path": "../../src/domain.ts"
          }
        ]
      },
      {
        "id": "markings",
        "level": "Introduction",
        "title": "Read state, bookmarks, and tags",
        "objective": "Keep three independent kinds of article state distinct.",
        "paragraphs": [
          "Unread is Entry reading state. It is independent of organizational Tags. Collection unread counts summarize the reading state of member Entries; the collection itself is not marked read in the same sense as an article.",
          "Bookmarking an article assigns one of six palette colors: Red, Orange, Yellow, Green, Blue, or Violet. Removing its color removes the bookmark. There is no separate bookmark boolean. Recoloring changes the mark while retaining bookmark membership.",
          "Tags are optional organizational labels. An Entry can carry several; removing an assignment does not delete the tag definition. Saved includes color-marked articles and annotations, so it is broader than bookmarked articles alone."
        ],
        "example": "An unread article can be bookmarked Violet and tagged Research at the same time. Marking it read should not remove either the color or the tag.",
        "questions": [
          {
            "prompt": "A bookmarked article changes from Blue to Violet. What happens?",
            "answers": [
              {
                "text": "It remains bookmarked with Violet as its color.",
                "feedback": "Color expresses bookmark state; recoloring does not remove it.",
                "correct": true
              },
              {
                "text": "It becomes unbookmarked until a flag is set.",
                "feedback": "There is no independent bookmark flag.",
                "correct": false
              },
              {
                "text": "Its reading state becomes unread.",
                "feedback": "Bookmark color and reading state are independent.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Removing Research from one article should do what?",
            "answers": [
              {
                "text": "Remove that assignment while preserving the definition and the article.",
                "feedback": "Assignment removal is distinct from deleting a definition.",
                "correct": true
              },
              {
                "text": "Delete Research everywhere.",
                "feedback": "Global deletion is a separate confirmed operation.",
                "correct": false
              },
              {
                "text": "Remove its bookmark color too.",
                "feedback": "Tags and bookmark color are independent.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Reading state, bookmarks, and color",
            "path": "../architecture/organization_model.md#reading-state-bookmarks-and-color"
          },
          {
            "title": "Tags",
            "path": "../architecture/organization_model.md#tags"
          },
          {
            "title": "Bookmark commands",
            "path": "../../src-tauri/src/commands/bookmarks.rs"
          }
        ]
      },
      {
        "id": "context",
        "level": "Applied",
        "title": "Preserve the reading context",
        "objective": "Recognize when the list changes and when only an article’s details change.",
        "paragraphs": [
          "Xaga preserves the current browsing context while a reader works. Marking an article as read changes how it appears without removing it from the list, even when you are looking at unread articles. Bookmark changes follow the same principle, including Saved lists.",
          "Explicit navigation or a filter change establishes a new list. This separates updating a visible item from reevaluating which items belong in the current view. It prevents an action on the current article from making that article disappear unexpectedly.",
          "Unpinning removes a navigation shortcut while preserving the open list and reader. Deleting the underlying content is a different action; the list-context rule does not define deletion behavior."
        ],
        "example": "Open an Unread list, read its first article, then unpin the collection. The article stays in the list, and you can keep reading. Reopening the Unread scope can produce a fresh membership list.",
        "questions": [
          {
            "prompt": "You’re looking at your unread articles and open one to read it. Xaga marks it as read. Should it disappear from the list, or stay visible?",
            "answers": [
              {
                "text": "Keep the article in the list, but show that it has been read.",
                "feedback": "The current list is preserved until an explicit membership refresh or navigation.",
                "correct": true
              },
              {
                "text": "Remove the article from the list immediately.",
                "feedback": "That would violate the documented context-preservation behavior.",
                "correct": false
              },
              {
                "text": "Ignore the state change until restart.",
                "feedback": "The article should show that it has been read.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "What does unpinning an open Topic change?",
            "answers": [
              {
                "text": "Its shortcut membership, while the open reading scope remains.",
                "feedback": "Navigation context does not depend on the pin continuing to exist.",
                "correct": true
              },
              {
                "text": "The Topic’s article membership.",
                "feedback": "Pinning organizes navigation, not content membership.",
                "correct": false
              },
              {
                "text": "The reader must close.",
                "feedback": "The current reader and list context are preserved.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "ListView behavior",
            "path": "../architecture/list_view.md"
          },
          {
            "title": "Navigation state",
            "path": "../../src/hooks/useRouter.tsx"
          }
        ]
      },
      {
        "id": "shortcuts",
        "level": "Applied",
        "title": "Shortcuts reference their targets",
        "objective": "Design for unavailable targets without losing user intent.",
        "paragraphs": [
          "Pin membership belongs to the library’s central pins.yaml store. A pin is a typed reference to a target; titles and counts resolve from current targets. Sorting shortcuts is presentation state and does not rewrite the stored sequence.",
          "A target that cannot currently be resolved remains pinned. The UI describes that it cannot find the target, offers Retry and Remove shortcut, and does not claim the target was deleted. Bulk lists can omit unreadable targets, so absence alone is insufficient evidence.",
          "A pinned Site with zero Entries is still a valid empty browsing target. It is not an unavailable managed record. Target navigation and Retry remain useful in read-only mode; removing a pin requires write permission."
        ],
        "example": "A Topic fails to resolve after a library read. Preserve its reference so that restoration can make the same shortcut usable again.",
        "questions": [
          {
            "prompt": "A pinned Topic is absent from the current list of targets. What is the appropriate initial response?",
            "answers": [
              {
                "text": "Show an unavailable shortcut with Retry and explicit removal.",
                "feedback": "Resolution failure does not establish deletion, and the reference preserves intent.",
                "correct": true
              },
              {
                "text": "Silently delete the pin.",
                "feedback": "That discards user intent and prevents automatic resolution later.",
                "correct": false
              },
              {
                "text": "Create a new Topic with the same title.",
                "feedback": "A new object would not restore the original target identity.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A pinned Site has no matching Entries. How should it appear?",
            "answers": [
              {
                "text": "As a valid empty Site with zero counts.",
                "feedback": "Site membership is derived; zero members do not invalidate the target.",
                "correct": true
              },
              {
                "text": "As a permanently broken reference.",
                "feedback": "An empty Site remains navigable.",
                "correct": false
              },
              {
                "text": "As a reason to delete the Site file.",
                "feedback": "There is no independent Site file.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Pin-store contract",
            "path": "../architecture/pins.md"
          },
          {
            "title": "Site shortcuts",
            "path": "../architecture/sites.md#shortcuts"
          },
          {
            "title": "Shortcut projection",
            "path": "../../src/hooks/useShortcutTargets.ts"
          }
        ]
      },
      {
        "id": "tags",
        "level": "Applied",
        "title": "Tag identity survives a rename",
        "objective": "Distinguish naming, assignment, rename, merge, and deletion.",
        "paragraphs": [
          "A tag has a stable UUID identity and a display name. Name comparisons trim surrounding whitespace and lowercase the comparison value. They do not perform Unicode normalization or case folding: Research and RESEARCH match, while straße and strasse remain distinct.",
          "Renaming preserves identity. A collision offers a separate explicit merge confirmation. The merge uses the destination name, but retains the ID with more indexed Entry assignments; ties retain the destination ID. The visible name therefore does not determine the surviving identity.",
          "Merging cannot restore the original distinction between assignments and must say so prominently. Deleting a definition removes its assignments without deleting articles. Both global operations differ from toggling one article’s chip. Long-running transitions are checkpointed; their completion is based on indexed candidates, with a documented missed-file consequence."
        ],
        "example": "Research and Notes collide during a rename. Offer a merge decision naming both tags and the resulting name, rather than silently turning rename into a destructive consolidation.",
        "questions": [
          {
            "prompt": "Which pair resolves to the same definition under the documented name rules?",
            "answers": [
              {
                "text": "Research and RESEARCH.",
                "feedback": "Comparison lowercases trimmed values while preserving display spelling.",
                "correct": true
              },
              {
                "text": "straße and strasse.",
                "feedback": "Unicode lowercase is not case folding; this pair stays distinct.",
                "correct": false
              },
              {
                "text": "Any two names with matching visual appearance.",
                "feedback": "Visual similarity is not the specified comparison rule.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A rename collides with another tag. What should the UX require?",
            "answers": [
              {
                "text": "Separate merge confirmation explaining the irreversible loss of assignment distinction.",
                "feedback": "A collision changes the operation’s consequences and needs explicit confirmation.",
                "correct": true
              },
              {
                "text": "Silent merge because the destination name is known.",
                "feedback": "The name alone does not authorize consolidation.",
                "correct": false
              },
              {
                "text": "Deletion of the articles carrying the source tag.",
                "feedback": "Tag operations do not delete articles.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Tag naming and transitions",
            "path": "../architecture/organization_model.md#tags"
          },
          {
            "title": "Tag storage",
            "path": "../architecture/tags.md"
          },
          {
            "title": "Registry implementation",
            "path": "../../crates/xaga-core/src/tags.rs"
          }
        ]
      },
      {
        "id": "capabilities",
        "level": "Applied",
        "title": "Read-only and embeddings are separate",
        "objective": "Describe degraded capabilities without conflating their causes.",
        "paragraphs": [
          "Library write ownership and embedding availability are independent. A read-only library refuses authoritative writes but can read existing data. A read-write library can still lose semantic capabilities if the model or embedding database fails to initialize.",
          "Embedding startup has loading, available, and unavailable states. The shell waits while initialization is loading; a failure opens the shell with embedding-backed features withheld. Semantic search, related lookup, and relevance ranking require both model and database resources.",
          "Read-only mode can result from lock contention or library-version metadata that is absent, mismatched, incomplete, or unreadable. Opening such a library does not silently upgrade it. Explain the capability and appropriate next action rather than treating every disabled control as the same failure."
        ],
        "example": "The model fails verification in a read-write instance. Ordinary file-backed workflows remain usable; semantic features are unavailable. This failure does not revoke the library writer.",
        "questions": [
          {
            "prompt": "Can a read-only instance have working semantic search?",
            "answers": [
              {
                "text": "Yes, when its existing read-only database and query model initialize.",
                "feedback": "Write permission and embedding readiness are independent axes.",
                "correct": true
              },
              {
                "text": "No; all search requires a library writer.",
                "feedback": "Read-only instances have a query-only initialization path.",
                "correct": false
              },
              {
                "text": "Only after silently upgrading the library.",
                "feedback": "Normal opening does not perform migrations.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Model verification fails in a read-write instance. Which description is accurate?",
            "answers": [
              {
                "text": "Semantic features are unavailable; library mode remains read-write.",
                "feedback": "Embedding failure does not change write ownership.",
                "correct": true
              },
              {
                "text": "The library is now automatically read-only.",
                "feedback": "Library mode is derived from the writer, not model readiness.",
                "correct": false
              },
              {
                "text": "The shell must wait forever.",
                "feedback": "Unavailable is a terminal startup state that permits the shell to open.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Embedding startup and runtime capability",
            "path": "../architecture/security/embedding_model_trust.md#runtime-failure-behavior"
          },
          {
            "title": "Library opening",
            "path": "../architecture/library_version.md#runtime-opening"
          },
          {
            "title": "Startup provider",
            "path": "../../src/providers/EmbeddingStartupProvider.tsx"
          }
        ]
      },
      {
        "id": "privacy",
        "level": "Advanced",
        "title": "Privacy changes are product changes",
        "objective": "Evaluate compatibility and performance proposals against Xaga’s privacy commitments.",
        "paragraphs": [
          "Xaga’s secure-default principle makes the secure behavior the initial behavior. When security conflicts with usability, the user controls the trade-off and receives an explanation of its consequences. A compatibility problem is not permission to relax a default invisibly.",
          "Feed fetch clients are built per invocation; article loaders have deliberately limited lifetimes. Reuse could retain server-writable state such as session tickets, validators, or cookies and make later activity linkable. Pooling and conditional GET therefore require a privacy review, not just a speed comparison.",
          "Network policy intends containment, minimal truthful disclosure, and bounded resources. The fetching document also records WebKit loopback proxy bypasses. Product language must distinguish those intentions and implemented protections from an unproven claim that every request is contained."
        ],
        "example": "A proposal to reuse loader webviews may improve load time. Its assessment must also identify the state that survives and which tracking channels that reopens.",
        "questions": [
          {
            "prompt": "A slower fetch could be sped up by retaining server validators. How should that proposal be framed?",
            "answers": [
              {
                "text": "As a performance and privacy trade-off requiring an explicit decision.",
                "feedback": "Validators can be server-writable tracking state.",
                "correct": true
              },
              {
                "text": "As a behavior-neutral optimization.",
                "feedback": "Resource lifetime and state retention affect privacy behavior.",
                "correct": false
              },
              {
                "text": "As proof that the privacy defaults are defective.",
                "feedback": "A cost is part of a trade-off, not sufficient evidence to discard the contract.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Which security statement is supported by the fetching document?",
            "answers": [
              {
                "text": "The proxy enforces its policy on requests it receives, with documented WebKit bypass gaps.",
                "feedback": "This preserves the distinction between policy and demonstrated enforcement.",
                "correct": true
              },
              {
                "text": "All WebKit network traffic is proven to pass through the proxy.",
                "feedback": "Native diagnostics demonstrate loopback bypasses.",
                "correct": false
              },
              {
                "text": "Successful sanitization guarantees that remote ideas are trustworthy.",
                "feedback": "Sanitization does not establish truth or network containment.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Secure defaults",
            "path": "../principles/secure_by_default.md"
          },
          {
            "title": "Resource lifetime and privacy",
            "path": "../architecture/security/performance_tradeoff.md"
          },
          {
            "title": "Fetching policy and enforcement gaps",
            "path": "../architecture/security/fetching_remote_content.md"
          }
        ]
      },
      {
        "id": "scale",
        "level": "Advanced",
        "title": "Read performance evidence at its actual scope",
        "objective": "Assess a feature at Xaga’s design target without overstating measurements.",
        "paragraphs": [
          "Xaga’s expected intake is roughly 240–360 thousand Entries per year, and its design target is around four times expected intake: a multi-million-Entry corpus. A demonstration with a small library is useful for interaction review but does not establish behavior at that target.",
          "The design-scale document separates measured fixtures, modeled extrapolations, and unmeasured costs. Its query timings can exclude rendering, IPC, mapping, and sorting. An index can accelerate selection while returning hundreds of thousands of summaries still takes substantial time and memory.",
          "Virtualization limits visible list items, not the size of the query result or client allocation. Product acceptance criteria should name the workflow, population, result density, and measured boundary. Record unsupported assumptions as facts with locations rather than silently deciding they are acceptable."
        ],
        "example": "A selective tag query performs well at 64 thousand rows. That does not establish the latency of an empty-selection list returning two million summaries in the UI.",
        "questions": [
          {
            "prompt": "What does list virtualization establish?",
            "answers": [
              {
                "text": "A bound on visible list items, not on query payload or total client memory.",
                "feedback": "The current API can still return the entire matching result.",
                "correct": true
              },
              {
                "text": "A guarantee of constant memory for every library size.",
                "feedback": "The full result can be allocated independently of visible list items.",
                "correct": false
              },
              {
                "text": "That query indexes are unnecessary.",
                "feedback": "Rendering and selection are different costs.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A document models million-row latency from a smaller fixture. How should training present it?",
            "answers": [
              {
                "text": "As an estimate with its assumptions and excluded costs.",
                "feedback": "Modeled scenarios are not measured end-to-end commitments.",
                "correct": true
              },
              {
                "text": "As a measured production guarantee.",
                "feedback": "That would exceed the evidence.",
                "correct": false
              },
              {
                "text": "As irrelevant because most examples are small.",
                "feedback": "The declared target governs design assessment.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Design target and measurement limits",
            "path": "../architecture/design_scale.md"
          },
          {
            "title": "Tag query scaling",
            "path": "../architecture/design_scale.md#tag-query-scaling"
          },
          {
            "title": "List rendering",
            "path": "../../src/components/list/EntityList.tsx"
          }
        ]
      },
      {
        "id": "review",
        "level": "Advanced",
        "title": "Review a cross-track product proposal",
        "objective": "Combine domain, context, accessibility, and durability in one decision.",
        "paragraphs": [
          "Consider a proposal that removes read articles immediately, deletes unresolvable shortcuts, and silently merges colliding tags to make the interface “cleaner.” Each simplification crosses a documented contract: reading context, retained pin intent, or explicit irreversible merge confirmation.",
          "A Xaga review starts by identifying the affected source of truth and user action. Determine whether the proposal changes navigation, assignment, identity, or underlying content. Similar-looking controls can have very different consequences.",
          "Present the conflict, the alternatives, and evidence to the product decision owner. Keep durable documentation aligned with any approved change. Transient stories specify remaining work and completion conditions; they do not become permanent authority for changed product behavior."
        ],
        "example": "For a new bulk organization action, trace its effects on tag definitions, article assignments, visible list membership, read-only controls, interruption, and feedback before evaluating its visual simplicity.",
        "questions": [
          {
            "prompt": "Which response best reviews the “cleaner interface” proposal?",
            "answers": [
              {
                "text": "Identify the three contract changes and present their consequences for an explicit decision.",
                "feedback": "The proposal affects user intent and state, not just appearance.",
                "correct": true
              },
              {
                "text": "Implement it because fewer items in the list always means simpler UX.",
                "feedback": "Visual reduction does not override the documented contracts.",
                "correct": false
              },
              {
                "text": "Keep all implementation details out of the review, including data loss.",
                "feedback": "Consequences such as irreversible merge are necessary to the decision.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Where should the lasting behavior of an approved change be recorded?",
            "answers": [
              {
                "text": "In the relevant durable docs, with a transient story containing only remaining work.",
                "feedback": "The documentation convention separates lasting contracts from task lifecycle.",
                "correct": true
              },
              {
                "text": "Only in an active task forever.",
                "feedback": "Durable docs must not depend on transient tasks.",
                "correct": false
              },
              {
                "text": "Only in a screenshot.",
                "feedback": "A screenshot does not specify ownership, interruption, or behavior.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Decision ownership",
            "path": "../principles/decision_ownership.md"
          },
          {
            "title": "Documentation conventions",
            "path": "../conventions/documentation.md"
          },
          {
            "title": "Organization model",
            "path": "../architecture/organization_model.md"
          }
        ]
      }
    ]
  },
  "interface": {
    "title": "UX designer / UI developer",
    "description": "Connect interaction design to Xaga’s state, platform port, and rendering boundaries.",
    "lessons": [
      {
        "id": "port",
        "level": "Introduction",
        "title": "The UI talks through Platform",
        "objective": "Locate the presentation-to-native boundary in Xaga.",
        "paragraphs": [
          "Xaga’s presentation layer is React and TypeScript. Platform is its declared port for native operations and subscriptions. TauriPlatform implements that interface and adapts native payloads to domain objects. main.tsx currently constructs the Tauri implementation and supplies it through PlatformContext.",
          "A component should express the operation through the port rather than introducing a direct native shortcut. This keeps the cross-layer contract visible and reviewable. Adding a Platform method widens that contract and should be justified by a real consumer.",
          "A browser-only mock can implement the same contract for fixtures, but merely running the Vite server does not replace Tauri APIs. The entrypoint also uses the native current-window API. A future capture mode must deliberately handle those startup dependencies; it is not an existing runtime promise."
        ],
        "example": "Trace a bookmark action from its component through Platform and TauriPlatform to the bookmark command. The port is the handoff, not a second persistence owner.",
        "questions": [
          {
            "prompt": "Where should a presentation component express a backend operation?",
            "answers": [
              {
                "text": "Through the Platform interface.",
                "feedback": "The platform port is the declared cross-layer door.",
                "correct": true
              },
              {
                "text": "Through a new direct invoke in every component.",
                "feedback": "That distributes native knowledge beyond the adapter.",
                "correct": false
              },
              {
                "text": "By editing the library Markdown from the browser.",
                "feedback": "Library writes belong to the native writer path.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Does starting Vite alone establish a backend-free Xaga UI?",
            "answers": [
              {
                "text": "No; native startup dependencies and the platform implementation still need a deliberate replacement.",
                "feedback": "main.tsx constructs TauriPlatform and reads the native window label.",
                "correct": true
              },
              {
                "text": "Yes; TypeScript interfaces automatically supply mock values.",
                "feedback": "An interface has no runtime implementation.",
                "correct": false
              },
              {
                "text": "Yes; the training HTML already replaces the application backend.",
                "feedback": "The training page is independent of the React application.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Platform port",
            "path": "../../src/platform/Platform.ts"
          },
          {
            "title": "Tauri adapter",
            "path": "../../src/platform/TauriPlatform.ts"
          },
          {
            "title": "Application startup",
            "path": "../../src/main.tsx"
          },
          {
            "title": "Least visibility",
            "path": "../principles/least_visibility_principle.md"
          }
        ]
      },
      {
        "id": "identity",
        "level": "Introduction",
        "title": "Shapes, roles, and loaded content",
        "objective": "Use Xaga’s domain contracts without fabricating fields.",
        "paragraphs": [
          "Entity provides id and title. ManagedEntity adds a managed lifecycle. Collection adds summary counts and a Feed, Topic, or Site kind. Feed and Topic have managed membership; Site only needs its derived identity and summary counts.",
          "An Article is a content-bearing Entry, retaining the entry kind and adding content and annotations. An Annotation is a separate entity tied to its containing article; its content is the user’s comment and is not the Note entity. These distinctions determine navigation and inspection targets.",
          "Guards validate structural shape together with kind where required. A brand alone is insufficient, and adding counts to an Entry does not turn it into a Collection. Loaded member references can be incomplete, so members.length does not replace authoritative summary counts."
        ],
        "example": "A Site card can use id, title, entryCount, and unreadCount. It should not synthesize createdAt or a fully loaded members list to resemble a Topic.",
        "questions": [
          {
            "prompt": "A Feed has ten loaded references but a larger authoritative count. Which count belongs in its summary?",
            "answers": [
              {
                "text": "The supplied authoritative summary count.",
                "feedback": "Loaded members need not cover the complete collection.",
                "correct": true
              },
              {
                "text": "Always members.length.",
                "feedback": "That can undercount partially loaded membership.",
                "correct": false
              },
              {
                "text": "A fabricated average of the two values.",
                "feedback": "The values have different meanings; averaging has no contract.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "What identifies an Article in the UI domain?",
            "answers": [
              {
                "text": "Entry shape and kind, plus content and annotations.",
                "feedback": "An Article remains an Entry with loaded content.",
                "correct": true
              },
              {
                "text": "A new article brand replacing entry.",
                "feedback": "The Article type retains the entry kind.",
                "correct": false
              },
              {
                "text": "Any object containing a title.",
                "feedback": "Entity shape alone is not enough.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Domain contracts and guards",
            "path": "../../src/domain.ts"
          },
          {
            "title": "Type identity",
            "path": "../architecture/type_identity.md"
          },
          {
            "title": "Collections",
            "path": "../architecture/organization_model.md#collections-and-identity"
          }
        ]
      },
      {
        "id": "motion",
        "level": "Introduction",
        "title": "Build the static experience first",
        "objective": "Apply Xaga’s motion convention to controls and overlays.",
        "paragraphs": [
          "Xaga starts with a complete, usable static interface. Add animation only inside prefers-reduced-motion: no-preference. When the user requests reduced motion, or the feature is unsupported, the base state remains usable without animation.",
          "The reload icon demonstrates this: base styles set animation to none and the no-preference query adds spinning to the busy state. The operation’s status is conveyed independently through its title and screen-reader feedback.",
          "The rule also covers transitions and component-library behavior. The Inspector drawer and tab indicator must respect the preference. When a component prevents the convention, surface that limitation and alternatives rather than assuming its defaults are compliant."
        ],
        "example": "A busy control needs a static status such as “Reload in progress...” even when a spinner is available. The spinner adds a visual cue; it does not own the state.",
        "questions": [
          {
            "prompt": "Which CSS approach follows Xaga’s convention?",
            "answers": [
              {
                "text": "Static base styles with animation enabled only inside no-preference.",
                "feedback": "Unsupported media queries then retain a usable static state.",
                "correct": true
              },
              {
                "text": "Animation by default, disabled only under reduce.",
                "feedback": "An unsupported feature would retain animation.",
                "correct": false
              },
              {
                "text": "Use JavaScript to force animation for all users.",
                "feedback": "That bypasses the user’s motion preference.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "What must remain when a busy animation is absent?",
            "answers": [
              {
                "text": "Meaningful static and screen-reader status feedback.",
                "feedback": "Motion cannot be the only communication channel.",
                "correct": true
              },
              {
                "text": "No visible indication until completion.",
                "feedback": "That leaves the static interface incomplete.",
                "correct": false
              },
              {
                "text": "A permanently disabled control with no explanation.",
                "feedback": "Disabling alone does not explain the operation’s state.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Motion convention",
            "path": "../conventions/motion.md"
          },
          {
            "title": "Reload icon implementation",
            "path": "../../src/components/content/Toolbar.module.css"
          },
          {
            "title": "Inspector motion and focus",
            "path": "../architecture/article_inspector.md#responsive-presentation-and-keyboard-access"
          }
        ]
      },
      {
        "id": "routing",
        "level": "Applied",
        "title": "Separate the list you are browsing from the article you select",
        "objective": "Preserve navigation intent through updates and async results.",
        "paragraphs": [
          "The router carries the browsing scope separately from the selected Entry or Annotation. An article shortcut establishes articleScopeId, while entryId and annotationId identify what the reader is showing. Selecting an annotation should not redefine the originating list.",
          "Reading-state and color events update the articles already shown without changing which articles are in the list. Unpinning a target does not clear an open list. Explicit scope or filter changes establish a fresh query. These rules are easy to break by placing every event in a generic reload handler.",
          "Async results must be associated with the scope that requested them. A superseded Tags query cannot replace a newer scope’s list. Route state also preserves selected tag IDs through back and forward navigation; those IDs are not persisted library settings."
        ],
        "example": "Open a pinned article’s annotation list, then select an annotation. The reader navigates to the parent article and passage; the list still belongs to the pinned article scope.",
        "questions": [
          {
            "prompt": "Which action should keep the current list membership?",
            "answers": [
              {
                "text": "A bookmark color change on a visible Entry.",
                "feedback": "The article’s bookmark changes, but the same articles stay in the list.",
                "correct": true
              },
              {
                "text": "An explicit change to the collection scope.",
                "feedback": "Scope navigation establishes a new list.",
                "correct": false
              },
              {
                "text": "A change to the selected tag set.",
                "feedback": "That selection explicitly establishes a new Tags query.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A slow old query finishes after a newer route query. What should happen?",
            "answers": [
              {
                "text": "Discard its result if it no longer belongs to the active scope.",
                "feedback": "Older work cannot overwrite newer navigation intent.",
                "correct": true
              },
              {
                "text": "Always show the last response to arrive.",
                "feedback": "Arrival order does not represent user intent.",
                "correct": false
              },
              {
                "text": "Persist both scopes in the library manifest.",
                "feedback": "These are navigation concerns, not library metadata.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "List and article-shortcut behavior",
            "path": "../architecture/list_view.md"
          },
          {
            "title": "Router",
            "path": "../../src/hooks/useRouter.tsx"
          },
          {
            "title": "List implementation",
            "path": "../../src/components/list/EntityList.tsx"
          }
        ]
      },
      {
        "id": "inspector",
        "level": "Applied",
        "title": "One Inspector, explicit targets",
        "objective": "Keep reader navigation, inspection targets, and responsive presentation distinct.",
        "paragraphs": [
          "The shared Inspector offers Related and Topics for Articles and Annotations, plus Tags for Articles. Article navigation and Annotation navigation do not automatically open a closed drawer. Inspect is an explicit action, and inspecting an Annotation remains distinct from inspecting its parent Article.",
          "Article inspection uses a 280-pixel panel when the reader container is at least 700 CSS pixels wide; narrower containers use a drawer. This is a reader-container breakpoint, not a viewport breakpoint. Annotation inspection always uses the drawer, including on wide layouts.",
          "Opening inspection preserves reading position. The modal has a name, close action, Escape dismissal, focus containment, and focus return to its opener with a reader fallback. Article and Annotation view selections are independent session state; tab children remain mounted while the Inspector is open."
        ],
        "example": "A wide browser can still have a narrow reader because surrounding panes consume space. Decide Article inspection presentation from reader width, not window width.",
        "questions": [
          {
            "prompt": "A wide layout opens Annotation inspection. Where should it appear?",
            "answers": [
              {
                "text": "In the Inspector drawer.",
                "feedback": "Annotation inspection always uses the drawer.",
                "correct": true
              },
              {
                "text": "In the Article side panel by replacing its target.",
                "feedback": "The Article panel and Annotation drawer have distinct targets.",
                "correct": false
              },
              {
                "text": "In a new browser window.",
                "feedback": "That is not the documented presentation.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Which measurement selects the Article panel presentation?",
            "answers": [
              {
                "text": "Reader-container width of at least 700 CSS pixels.",
                "feedback": "The available reader space owns the breakpoint.",
                "correct": true
              },
              {
                "text": "A viewport width of at least 700 pixels regardless of panes.",
                "feedback": "Viewport width can overstate the space available to the reader.",
                "correct": false
              },
              {
                "text": "The width of the selected article’s images.",
                "feedback": "Content dimensions do not select the Inspector layout.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Inspector contract",
            "path": "../architecture/article_inspector.md"
          },
          {
            "title": "Inspector implementation",
            "path": "../../src/components/content/Inspector.tsx"
          },
          {
            "title": "Reader target and presentation",
            "path": "../../src/components/content/EntityView.tsx"
          }
        ]
      },
      {
        "id": "committed",
        "level": "Applied",
        "title": "Project committed intent into UI state",
        "objective": "Show successful writes promptly without creating a second authority.",
        "paragraphs": [
          "The Topics provider maintains a library-session projection of committed membership changes. It overlays successful adds and removes on a reverse index that may lag. It survives picker remounts but is discarded with the library session; files remain authoritative.",
          "Before the first Topics list arrives, the picker shows Loading Topics… rather than treating missing data as an empty list or offering creation prematurely. A failed write does not alter selected membership. Target changes must prevent old lookups or writes from replacing the new target’s display.",
          "Tag assignment follows the same committed-intent discipline: the Article Tags view reads file assignments, and successful writes emit a typed EntryTagChange. Chips expose aria-pressed and immediate assignment; there is no Save step. Read-only inspection remains visible with mutations disabled."
        ],
        "example": "A Topic add succeeds while the reverse index is catching up. Keep the committed selection visible across a picker remount by applying the session projection, rather than reverting to a stale lookup.",
        "questions": [
          {
            "prompt": "A membership write fails. What should the picker project?",
            "answers": [
              {
                "text": "The last committed membership, with the failure reported.",
                "feedback": "Failed writes do not produce committed deltas.",
                "correct": true
              },
              {
                "text": "The requested membership as if it succeeded.",
                "feedback": "That would manufacture a successful state.",
                "correct": false
              },
              {
                "text": "An empty Topics list.",
                "feedback": "A write failure does not invalidate every available Topic.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "The Topics provider has not loaded its first list. What belongs in the view?",
            "answers": [
              {
                "text": "Loading Topics… until the initial list resolves.",
                "feedback": "Unloaded data must not be presented as an empty collection.",
                "correct": true
              },
              {
                "text": "Create a New Topic as proof none exist.",
                "feedback": "The absence of a response is not evidence of no Topics.",
                "correct": false
              },
              {
                "text": "A persisted copy of whichever library was opened last.",
                "feedback": "Projection state must not leak between libraries.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Inspector membership projection",
            "path": "../architecture/article_inspector.md#topic-membership"
          },
          {
            "title": "Topics provider",
            "path": "../../src/hooks/useTopics.tsx"
          },
          {
            "title": "Shared membership UI",
            "path": "../../src/components/content/TopicMembership.tsx"
          },
          {
            "title": "Tag assignment API",
            "path": "../architecture/tags.md#entry-assignment-api"
          }
        ]
      },
      {
        "id": "tags-view",
        "level": "Applied",
        "title": "Make tag browsing a route operation",
        "objective": "Apply intersection semantics and preserve selected identity.",
        "paragraphs": [
          "Multiple selected Tags mean intersection: an Entry must carry every selected identity. Empty selection shows all Entries under the active filters rather than the mixed Entry/Annotation All items shortcut. Text filtering and Saved/All remain available.",
          "With selected Tags, the Unread control is hidden and a retained Unread filter is treated as All. Clearing the set restores the control and retained filter. Sidebar and responsive header selections share route IDs and support keyboard interaction.",
          "Renames change labels without changing selected IDs or refreshing membership. If a selected definition disappears from a registry refresh, show “Unavailable tag” and let the user deselect it; do not silently clear the selection. Explicit Delete removes the selection but retains the current list until the next explicit refresh or navigation."
        ],
        "example": "Select Research and History. The list contains Entries tagged with both. Renaming Research changes its label; it must not broaden the query by dropping that selected identity.",
        "questions": [
          {
            "prompt": "Selecting two Tags means which membership?",
            "answers": [
              {
                "text": "Entries carrying both selected identities.",
                "feedback": "Tags browsing uses intersection, not union.",
                "correct": true
              },
              {
                "text": "Entries carrying either selected identity.",
                "feedback": "That would implement a different query contract.",
                "correct": false
              },
              {
                "text": "Every Annotation in either Topic.",
                "feedback": "Tags browsing initially covers Entries, and Tags are not Topics.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A selected tag is absent from a registry snapshot. What should the UI do?",
            "answers": [
              {
                "text": "Retain a deselectable Unavailable tag selection.",
                "feedback": "The selection must not silently broaden to fewer constraints.",
                "correct": true
              },
              {
                "text": "Show its UUID as the user-facing name.",
                "feedback": "The contract explicitly avoids exposing internal IDs as labels.",
                "correct": false
              },
              {
                "text": "Clear all selected Tags.",
                "feedback": "That would change the user’s query without their action.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Tags browsing contract",
            "path": "../architecture/list_view.md#tag-browsing"
          },
          {
            "title": "Entry query and tag storage",
            "path": "../architecture/tags.md"
          },
          {
            "title": "Router state",
            "path": "../../src/hooks/useRouter.tsx"
          }
        ]
      },
      {
        "id": "rendering",
        "level": "Advanced",
        "title": "Stored content is still untrusted",
        "objective": "Trace the read-time rendering boundary and its separate controls.",
        "paragraphs": [
          "Content read from storage is untrusted even if ingest sanitized it. External edits, historical versions, imports, and user-authored fields all cross the read boundary. The rendering path must apply its own controls rather than trusting provenance.",
          "markdownPipeline parses Markdown and embedded HTML, then sanitizes rendered output with DOMPurify. buildEntryHtml prepares the reader document. HtmlReader presents it inside the configured iframe boundary. Sanitization, iframe restrictions, and CSP address different parts of the exposure.",
          "Trusted UI network restrictions are also distinct from rendering sanitation. The security docs record native loopback proxy bypasses and separate CSP probes. Passing a browser-level sanitizer check does not prove native WebKit routing, resource persistence, or complete packaged-app isolation."
        ],
        "example": "A locally edited article contains an event-handler attribute. Its presence on disk does not authorize execution in the reader; read-time output sanitation still applies.",
        "questions": [
          {
            "prompt": "Why sanitize a saved article again during rendering?",
            "answers": [
              {
                "text": "Storage is an input boundary and ingest-time checks are not sufficient authority.",
                "feedback": "The same read-time controls apply regardless of original provenance.",
                "correct": true
              },
              {
                "text": "Only because the article came from a remote feed.",
                "feedback": "Local and user-authored persisted content are covered too.",
                "correct": false
              },
              {
                "text": "To prove every claim in the article is true.",
                "feedback": "Sanitization controls executable markup, not factual content.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A DOM test proves an unsafe HTML attribute is removed. What does it establish?",
            "answers": [
              {
                "text": "That tested sanitizer behavior, not complete native network isolation.",
                "feedback": "The native engine and other boundaries require their own evidence.",
                "correct": true
              },
              {
                "text": "That WebKit can never reach loopback.",
                "feedback": "The native docs record bypass observations.",
                "correct": false
              },
              {
                "text": "That the model’s installed bytes are verified.",
                "feedback": "Model integrity is a separate backend boundary.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Read-time content handling",
            "path": "../architecture/security/handling_persisted_content.md"
          },
          {
            "title": "Markdown sanitation",
            "path": "../../src/text/markdownPipeline.ts"
          },
          {
            "title": "Reader HTML construction",
            "path": "../../src/text/buildEntryHtml.ts"
          },
          {
            "title": "Reader presentation",
            "path": "../../src/components/content/HtmlReader.tsx"
          }
        ]
      },
      {
        "id": "diagnostics",
        "level": "Advanced",
        "title": "Keep failure feedback attached to its attempt",
        "objective": "Preserve readable content, notification ownership, and diagnostic limits.",
        "paragraphs": [
          "An article load attempt has a diagnostic ID shared across frontend and native stages. Error Details opens that attempt’s record, and Copy diagnostics copies JSON as plain text. Technical details are not substituted into the ordinary user-facing message or rendered as HTML.",
          "The error sink queues notifications rather than allowing later warnings to replace the current toast and its Details target. Diagnostic toasts remain until dismissed; opening or closing Details does not dismiss the toast. Saved content remains displayed during retrieval and after retrieval failure.",
          "Navigation cancels the frontend attempt, and a stale completion must not reset another article’s reload state. Native history and logs are bounded, so missing events are not proof that work never happened. Image-enabled and image-attempted are separate observations, and page-reported events have stated limits."
        ],
        "example": "Article A is reloading when the user opens Article B. A’s later completion must not clear B’s busy state or replace B’s current presentation.",
        "questions": [
          {
            "prompt": "A favicon warning arrives while a diagnostic error toast is visible. What should happen?",
            "answers": [
              {
                "text": "Queue the accepted notification without replacing the current toast’s Details target.",
                "feedback": "The sink preserves arrival order and each notification’s identity.",
                "correct": true
              },
              {
                "text": "Replace the error with the latest message.",
                "feedback": "That could disconnect the user from the relevant attempt.",
                "correct": false
              },
              {
                "text": "Render the warning’s technical payload as HTML.",
                "feedback": "Diagnostic data is presented and copied as plain text.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Diagnostics say image capture was enabled. What can you conclude?",
            "answers": [
              {
                "text": "The preference allowed capture; this alone does not establish an attempted or completed fetch.",
                "feedback": "The diagnostic vocabulary distinguishes preference, attempt, and result.",
                "correct": true
              },
              {
                "text": "Every article image was downloaded.",
                "feedback": "Enabled is not a result.",
                "correct": false
              },
              {
                "text": "No image could have failed.",
                "feedback": "The preference says nothing about outcomes.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Article diagnostics contract",
            "path": "../architecture/article_loading_diagnostics.md"
          },
          {
            "title": "Frontend attempt ownership",
            "path": "../../src/utils/ArticleLoadAttempt.ts"
          },
          {
            "title": "Native diagnostic collector",
            "path": "../../src-tauri/src/article_diagnostics.rs"
          }
        ]
      },
      {
        "id": "review",
        "level": "Advanced",
        "title": "Review an interface change across boundaries",
        "objective": "Use Xaga-specific evidence to assess a UI change beyond its happy path.",
        "paragraphs": [
          "A strong Xaga interface review traces target identity, list scope, authoritative state, in-flight work, and focus. A change can look correct in one screenshot while losing an Annotation target, applying an old query, or remounting the reader when an Inspector opens.",
          "Use small deterministic scenarios to distinguish unloaded, empty, unavailable, read-only, and failed states. Those states carry different meanings and affordances. Do not reduce them to a single disabled control or an empty list.",
          "Choose checks for the boundary changed: component tests for target and focus behavior, route tests for superseded queries, and native checks for WebKit-specific claims. Keep helpers local until another consumer proves they need export, and use the established semantic vocabulary to make the operation clear."
        ],
        "example": "For an Inspector refactor, verify Article and Annotation targets, narrow and wide reader containers, keyboard dismissal and focus return, reduced motion, failed writes, and reader-position preservation.",
        "questions": [
          {
            "prompt": "Which review scenario can expose an identity bug a static screenshot misses?",
            "answers": [
              {
                "text": "Switch inspected targets while the first lookup is still pending.",
                "feedback": "It tests whether stale target work can overwrite the current target.",
                "correct": true
              },
              {
                "text": "Render only a fully loaded successful Article once.",
                "feedback": "That does not challenge ownership across asynchronous changes.",
                "correct": false
              },
              {
                "text": "Replace every empty state with a spinner.",
                "feedback": "That conceals state distinctions rather than verifying them.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A shared helper has only one file-local consumer. What visibility fits Xaga’s rule?",
            "answers": [
              {
                "text": "Keep it local until a concrete consumer proves an export is needed.",
                "feedback": "Least visibility limits coupling and preserves useful dead-code analysis.",
                "correct": true
              },
              {
                "text": "Export it now for hypothetical future use.",
                "feedback": "Speculative consumers do not justify wider visibility.",
                "correct": false
              },
              {
                "text": "Expose every helper through a new export-all barrel.",
                "feedback": "The convention explicitly rejects barrel sprawl.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Inspector contract",
            "path": "../architecture/article_inspector.md"
          },
          {
            "title": "Least visibility",
            "path": "../principles/least_visibility_principle.md"
          },
          {
            "title": "Semantic vocabulary",
            "path": "../conventions/vocabulary/semantic_vocabulary.md"
          },
          {
            "title": "Inspector behavior tests",
            "path": "../../src/components/content/EntityView.inspector.test.tsx"
          }
        ]
      }
    ]
  },
  "engineering": {
    "title": "Architect / Backend engineer",
    "description": "Trace authority, writes, jobs, recovery, and security through Xaga’s backend.",
    "lessons": [
      {
        "id": "layers",
        "level": "Introduction",
        "title": "Put orchestration at the command boundary",
        "objective": "Trace a user action through the native layers without moving their responsibilities.",
        "paragraphs": [
          "xaga-core owns pure logic and file APIs. The Tauri command layer orchestrates writes and downstream work. The CLI presents results, manages the database, and performs explicit migrations. A capability belongs according to what it does, not according to which caller first needed it.",
          "The bookmark commands first obtain the library writer, await the color mutation, and then enqueue SyncState with the committed color. The command knows the user action and carries it to consumers. Core does not decide which embedding job to enqueue.",
          "Putting progress printing into core would cross the boundary in the other direction. Shared logic may need a deliberate callback contract, but presentation-interleaved code does not become pure merely because two callers need it."
        ],
        "example": "For a new authoritative state change, identify the core file operation separately from write-then-enqueue sequencing and CLI output.",
        "questions": [
          {
            "prompt": "Where should a successful bookmark write enqueue SyncState?",
            "answers": [
              {
                "text": "In the command layer after the writer operation succeeds.",
                "feedback": "The command owns orchestration and carries committed state.",
                "correct": true
              },
              {
                "text": "Inside core’s file patcher.",
                "feedback": "Core does not own the embedding queue.",
                "correct": false
              },
              {
                "text": "In a file watcher that guesses the action.",
                "feedback": "The command already has the precise intent.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A CLI operation needs shared validation and progress printing. What belongs in core?",
            "answers": [
              {
                "text": "The validation logic, with presentation remaining at the caller.",
                "feedback": "Ownership follows the capability’s nature.",
                "correct": true
              },
              {
                "text": "All console formatting because the CLI needed it first.",
                "feedback": "That would import presentation into core.",
                "correct": false
              },
              {
                "text": "The CLI’s argument parsing and exit policy.",
                "feedback": "Those belong to the presentation surface.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Layer ownership",
            "path": "../architecture/layer_ownership.md"
          },
          {
            "title": "Carry intent end-to-end",
            "path": "../principles/carry_intent_end_to_end.md"
          },
          {
            "title": "Bookmark orchestration",
            "path": "../../src-tauri/src/commands/bookmarks.rs"
          }
        ]
      },
      {
        "id": "authority",
        "level": "Introduction",
        "title": "Files own data; LanceDB mirrors it",
        "objective": "Identify the authoritative representation before designing a write or rebuild.",
        "paragraphs": [
          "Entry Markdown owns content, reading state, color, Tags, and Annotations. Feed Atom files own feed membership; Topic Markdown owns ordered typed Topic membership. pins.yaml owns shortcut references, tags.yaml owns tag identities and transitions, and library.yaml owns completed and pending library versions.",
          "LanceDB is the queryable derived read layer. Its columns must be reconstructible from authoritative files. The standing producer rule names live command intent and recovery reconciliation; adding an unrelated cache writer would create drift.",
          "Direction matters. A Topic’s forward membership is read from its file; the reverse index answers which Topics contain an entity. Replacing the forward source with a convenient reverse projection would lose order, type, and Annotation reference information."
        ],
        "example": "A Topic membership change can leave an Entry content checksum unchanged. Recovery must rebuild the membership-derived structure independently rather than treating the content checksum as a universal change detector.",
        "questions": [
          {
            "prompt": "Where does authoritative ordered Topic membership live?",
            "answers": [
              {
                "text": "In the Topic Markdown file.",
                "feedback": "The reverse index is a derived direction of lookup.",
                "correct": true
              },
              {
                "text": "Only in each member’s cached topics column.",
                "feedback": "That projection cannot replace ordered typed membership.",
                "correct": false
              },
              {
                "text": "In whichever UI component loaded it last.",
                "feedback": "UI projections do not own durable membership.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Why is an unchanged content checksum insufficient to skip every recovery step?",
            "answers": [
              {
                "text": "Membership or state can change independently of content.",
                "feedback": "Different derived structures need gates that observe their own source changes.",
                "correct": true
              },
              {
                "text": "Checksums never have any value.",
                "feedback": "They are useful for the content they actually cover.",
                "correct": false
              },
              {
                "text": "The cache is the true source after first startup.",
                "feedback": "The cache remains reconstructible from files.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Single source of truth",
            "path": "../principles/single_source_of_truth.md"
          },
          {
            "title": "Recovery overview",
            "path": "../../crates/xaga-core/src/recovery/README.md"
          },
          {
            "title": "Topic commands",
            "path": "../../src-tauri/src/commands/topics.rs"
          }
        ]
      },
      {
        "id": "locks",
        "level": "Introduction",
        "title": "Separate process rights from file serialization",
        "objective": "Explain the library lock, writer lanes, and account lock as distinct scopes.",
        "paragraphs": [
          "The library’s exclusive advisory flock on xaga.lock permits at most one participating writing process. The runtime writer owns that right; writer absence is read-only mode. The OS releases handle-tied locks on exit, so an existing lock file is not evidence of a stale held lock.",
          "Inside the process, LibraryWriter serializes the entire read-modify-write for each file in a lane. Different files may run concurrently. Atomic rename avoids torn files but cannot prevent two independent operations from reading the same old version and overwriting each other.",
          "The account lock on settings.json protects machine/user-scoped resources such as settings and model download, independently of library choice. Its operation-scoped shared leases preserve exclusion across atomic inode replacement. Readers do not take it, and stale-view checks prevent settings writes from silently overwriting a view changed by another process."
        ],
        "example": "Two Annotation edits on one Article need the same file lane even though the process already owns the library lock. A model download needs account-scoped coordination even when its caller owns a library.",
        "questions": [
          {
            "prompt": "Why does a library-lock holder still need per-file lanes?",
            "answers": [
              {
                "text": "Its own concurrent read-modify-write operations can otherwise lose updates.",
                "feedback": "The process lock excludes other processes, not its own async operations.",
                "correct": true
              },
              {
                "text": "Atomic rename exposes half-written files.",
                "feedback": "Rename prevents torn replacement; lost updates are a separate problem.",
                "correct": false
              },
              {
                "text": "Each file needs a separate process.",
                "feedback": "Lanes provide in-process serialization without that requirement.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Two instances use different libraries but update settings. Which right coordinates them?",
            "answers": [
              {
                "text": "The account lock on settings.json.",
                "feedback": "Settings belong to machine/user scope, not either library.",
                "correct": true
              },
              {
                "text": "Only the first library’s xaga.lock.",
                "feedback": "A per-library lock cannot coordinate an independent account resource.",
                "correct": false
              },
              {
                "text": "No coordination because the library paths differ.",
                "feedback": "They still share settings and model resources.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Library lock and writer lanes",
            "path": "../architecture/single_writer_library_lock.md"
          },
          {
            "title": "Account lock and rotation",
            "path": "../architecture/single_writer_account_lock.md"
          },
          {
            "title": "Writer implementation",
            "path": "../../crates/xaga-core/src/library_writer/mod.rs"
          },
          {
            "title": "Atomic files and locks",
            "path": "../../crates/xaga-core/src/fs.rs"
          }
        ]
      },
      {
        "id": "priority",
        "level": "Applied",
        "title": "Priority describes who is waiting",
        "objective": "Keep job action, urgency, cost, and shutdown contract distinct.",
        "paragraphs": [
          "Embedding jobs describe actions; tiers describe precedence. The same action can ride different tiers depending on its producer and urgency. Control, Interactive, Regular, and Maintenance drain highest-first. Work is not low priority simply because it is expensive.",
          "A feed command’s visible row upsert can be Interactive while its later vector work is Regular. Recovery can schedule the same kind of embedding work as Maintenance. The question is how soon the result must reflect and for whom, not whether ONNX or many rows are involved.",
          "LibraryWriter has its own Interactive and Regular tiers with a shutdown contract: user-authored Interactive backlog must drain; re-derivable Regular backlog can be dropped. Do not confuse those writer admission rules with the embedding scheduler. The embedding service drops queued work on shutdown and relies on recovery for derived data."
        ],
        "example": "A heavy user action may be urgent, while a cheap background repair may be deferrable. Shutdown itself disproves “expensive means low priority.”",
        "questions": [
          {
            "prompt": "How should an embedding job’s tier be selected?",
            "answers": [
              {
                "text": "By importance and urgency: who needs the result and how soon.",
                "feedback": "Cost is a separate axis from precedence.",
                "correct": true
              },
              {
                "text": "By whether the action uses the model.",
                "feedback": "Action type does not define urgency.",
                "correct": false
              },
              {
                "text": "By assigning all bulk-shaped work to Maintenance.",
                "feedback": "A bulk user action can still need immediate visible results.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "What is safe to admit to the writer’s Regular queue?",
            "answers": [
              {
                "text": "Only re-derivable work compatible with its shutdown drop contract.",
                "feedback": "Regular backlog can be dropped during writer shutdown.",
                "correct": true
              },
              {
                "text": "An unsaved user Annotation because it is small.",
                "feedback": "User-authored state must drain; size does not make it recoverable.",
                "correct": false
              },
              {
                "text": "Any write, because recovery restores clobbered source content.",
                "feedback": "Recovery derives from files and cannot recreate lost user input.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Priority definition",
            "path": "../principles/definition_of_priority.md"
          },
          {
            "title": "Embedding queue and jobs",
            "path": "../../crates/xaga-core/src/embeddings/service.rs"
          },
          {
            "title": "Writer admission and shutdown",
            "path": "../architecture/single_writer_library_lock.md#in-process-write-serialization-the-writer-axis-2"
          }
        ]
      },
      {
        "id": "crashes",
        "level": "Applied",
        "title": "Define the crash between every commit",
        "objective": "Use ordering and observable recovery gates to explain partial states.",
        "paragraphs": [
          "Graceful shutdown and hard kill are different failure classes. Inline work in an embedding-service loop turn is uninterrupted by its cooperative Shutdown handling, but a hard kill can stop a multi-commit sequence between any commits. Queued derived work may be dropped; source files support later repair.",
          "Feed persistence commits Entry files before feed membership. Image resources land before Markdown referencing them. An orphaned resource is preferable to a newly committed reference without its bytes. These orderings define which partial states can survive rather than pretending a sequence is one transaction.",
          "State recovery compares read state and color independently of content checksums. Queued vector completion must preserve current metadata and skip stale or deleted Annotation rows. Every partial state needs a gate that can observe it; a checksum that excludes Annotation comments cannot detect a same-ID comment change by itself."
        ],
        "example": "The process dies after color is written to Markdown but before SyncState runs. Recovery reads the authoritative color and repairs the cache without needing to re-embed unchanged article text.",
        "questions": [
          {
            "prompt": "A hard kill occurs after a file state write and before cache sync. What is the intended repair path?",
            "answers": [
              {
                "text": "Recovery compares the file state and repairs the derived row.",
                "feedback": "The source survived; a state-specific gate detects the mismatch.",
                "correct": true
              },
              {
                "text": "Roll back the source to the stale cache automatically.",
                "feedback": "That would make the mirror authoritative.",
                "correct": false
              },
              {
                "text": "Assume the service loop made the sequence atomic.",
                "feedback": "Hard kill is not constrained by cooperative loop boundaries.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "An old Annotation vector job completes after the Annotation was deleted. What must it avoid?",
            "answers": [
              {
                "text": "Recreating the deleted row or restoring old metadata.",
                "feedback": "Completion is conditional on current row existence and matching checksum.",
                "correct": true
              },
              {
                "text": "Checking current row state.",
                "feedback": "That check is precisely what prevents resurrection.",
                "correct": false
              },
              {
                "text": "Changing only the vector of a valid matching pending row.",
                "feedback": "That is the allowed constrained update.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Crash consistency",
            "path": "../architecture/crash_consistency.md"
          },
          {
            "title": "Queued Annotation completion",
            "path": "../architecture/crash_consistency.md#queued-annotation-vector-completion"
          },
          {
            "title": "Persistence implementation",
            "path": "../../crates/xaga-core/src/persistence/articles.rs"
          }
        ]
      },
      {
        "id": "transitions",
        "level": "Applied",
        "title": "Checkpoint tag transitions outside the cache",
        "objective": "Bound work while preserving retry state and explicit completion semantics.",
        "paragraphs": [
          "Tag definitions and pending rename, merge, and delete transitions live in tags.yaml. Entry files own readable assignment labels; LanceDB stores resolved UUID assignments. Identity survives ordinary rename even while source labels are being rewritten.",
          "Batches persist pending work before source mutations and advance progress after successful work. Entry rewrites serialize the complete read-modify-write in Entry lanes; the registry uses its own writer lane. Revision checks reject stale checkpoint requests. Recovery resumes pending transitions before ordinary label discovery.",
          "Completion follows indexed candidates and persisted pending work without a corpus-wide source verification pass. A file missing from the index can retain an old label and later recreate it as a separate identity. Active reservations prevent conflicting operations during transition, but retired names are not permanent tombstones. This accepted limit must remain visible."
        ],
        "example": "Deleting Research must persist its reservation and pending batch before removing file labels and cached IDs. If interrupted, the pending IDs allow replay even after the cache is rebuilt.",
        "questions": [
          {
            "prompt": "When should a pending batch be persisted?",
            "answers": [
              {
                "text": "Before the source writes it describes.",
                "feedback": "Otherwise a crash could lose the evidence needed to resume.",
                "correct": true
              },
              {
                "text": "Only after the whole corpus finishes.",
                "feedback": "That provides no bounded restart point.",
                "correct": false
              },
              {
                "text": "Only in LanceDB because it is faster.",
                "feedback": "Retry state must survive rebuilding the derived cache.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A rename finishes its indexed batches but one file was absent from the index. What is the documented consequence?",
            "answers": [
              {
                "text": "Its old label can later be discovered as a new identity.",
                "feedback": "Completion does not include a full-source verification pass or lasting old-name chain.",
                "correct": true
              },
              {
                "text": "Completion proves every source file was checked.",
                "feedback": "That is explicitly outside the completion contract.",
                "correct": false
              },
              {
                "text": "The old label is permanently prohibited by a tombstone.",
                "feedback": "Retirement releases the reservation.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Tag transition contract",
            "path": "../architecture/tags.md"
          },
          {
            "title": "Checkpoint ordering",
            "path": "../architecture/crash_consistency.md#tag-definitions-and-transitions"
          },
          {
            "title": "Rename implementation",
            "path": "../../crates/xaga-core/src/tags/rename.rs"
          },
          {
            "title": "Merge implementation",
            "path": "../../crates/xaga-core/src/tags/merge.rs"
          }
        ]
      },
      {
        "id": "versions",
        "level": "Applied",
        "title": "Version the library, not just the app",
        "objective": "Trace explicit upgrades and safe read-only startup.",
        "paragraphs": [
          "Each library stores its completed application major.minor in library.yaml. Patch, prerelease, and build suffixes do not define a separate library version. Updating the app package does not relabel existing libraries, and absence of a manifest is not interpreted as a guessed historical format.",
          "Before 1.0, the CLI orchestrates explicit conversion under the library lock. It records a pending target before source or cache changes, preserves the completed version during conversion, and completes only after successful conversion and reconciliation. Retry must retain the in-progress target.",
          "App startup checks metadata before starting a writer or embedding service. Mismatched, absent, incomplete, or unreadable metadata opens read-only; errors preserve the original manifest. Read-only opening does not initialize a missing cache or perform recovery. Every storage-model change during 0.x requires a minor-version increment."
        ],
        "example": "A crash midway through conversion leaves completed version A and pending target B. The next upgrade retries B; normal opening must not declare B complete merely because some files already changed.",
        "questions": [
          {
            "prompt": "When may an explicit upgrade mark its target complete?",
            "answers": [
              {
                "text": "After conversion and cache reconciliation both succeed.",
                "feedback": "The manifest records completed work rather than optimistic intent.",
                "correct": true
              },
              {
                "text": "As soon as the target is selected.",
                "feedback": "Selection belongs in the pending marker.",
                "correct": false
              },
              {
                "text": "Whenever a new application binary opens the folder.",
                "feedback": "Normal opening does not migrate or relabel the library.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "What should normal startup do with missing version metadata?",
            "answers": [
              {
                "text": "Open read-only without inventing a historical version or initializing the manifest.",
                "feedback": "Unversioned is an explicit state requiring the upgrade path.",
                "correct": true
              },
              {
                "text": "Assume the oldest supported format and write it immediately.",
                "feedback": "Absence is not evidence for a particular historical version.",
                "correct": false
              },
              {
                "text": "Derive the library version from the most recently opened other library.",
                "feedback": "Each library owns its own metadata.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Library version contract",
            "path": "../architecture/library_version.md"
          },
          {
            "title": "Explicit upgrade",
            "path": "../library_upgrade.md"
          },
          {
            "title": "Release and storage versions",
            "path": "../development/release_cycle.md"
          },
          {
            "title": "Manifest API",
            "path": "../../crates/xaga-core/src/library/manifest.rs"
          }
        ]
      },
      {
        "id": "security",
        "level": "Advanced",
        "title": "Review authority at each input boundary",
        "objective": "Distinguish network containment, byte handling, and command authorization.",
        "paragraphs": [
          "The network proxy validates the destination addresses it resolves and pins connections to validated IPs. It rejects the whole destination when any result is disallowed. Redirects and subresources routed through it face the same policy. WebKit loopback bypass observations limit claims about end-to-end containment.",
          "Network control does not sanitize fetched bytes, and ingest sanitation does not replace read-time sanitation. Parsing, resource bounds, persisted-content rendering, and model verification are separate boundaries. Review each input at the point where it gains authority.",
          "Every registered Tauri command is a potential invoke surface. command_registry.json classifies commands and generates registration and permission data; native window checks and library-write gates remain separate checks. User-supplied filesystem-path operations belong in native Rust orchestration with the dialog and operation together, rather than handing path authority through JavaScript."
        ],
        "example": "An import menu opens its file dialog in Rust and calls a native operation directly. A URL-based stored-PDF opener derives its library path in Rust; it does not accept an arbitrary JS-selected disk path.",
        "questions": [
          {
            "prompt": "Why does the proxy reject a DNS result set containing both public and private addresses?",
            "answers": [
              {
                "text": "A public address must not smuggle a disallowed destination past validation.",
                "feedback": "All resolved addresses must satisfy policy before pinning the connection.",
                "correct": true
              },
              {
                "text": "Only the first answer matters because later answers cannot be used.",
                "feedback": "That does not preserve the intended containment rule.",
                "correct": false
              },
              {
                "text": "DNS validation also proves that page HTML is safe.",
                "feedback": "Destination policy and content sanitation are separate boundaries.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Where should a user-selected import file path be handled?",
            "answers": [
              {
                "text": "In native orchestration that owns both the dialog and the operation.",
                "feedback": "A JS-selected path on invoke would grant caller-controlled filesystem authority.",
                "correct": true
              },
              {
                "text": "In a generic webview-callable read-any-path command.",
                "feedback": "That would expose ambient filesystem authority.",
                "correct": false
              },
              {
                "text": "Only in CSS so it never appears in Rust.",
                "feedback": "CSS cannot authorize native filesystem access.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Fetching controls and gaps",
            "path": "../architecture/security/fetching_remote_content.md"
          },
          {
            "title": "Command surface",
            "path": "../architecture/security/command_surface.md"
          },
          {
            "title": "Command registry",
            "path": "../../src-tauri/command_registry.json"
          },
          {
            "title": "Native imports",
            "path": "../../src-tauri/src/native/feeds.rs"
          },
          {
            "title": "Persisted-content boundary",
            "path": "../architecture/security/handling_persisted_content.md"
          }
        ]
      },
      {
        "id": "model",
        "level": "Advanced",
        "title": "Integrity pins bytes, not benignity",
        "objective": "Explain model verification, anchoring, and failure behavior precisely.",
        "paragraphs": [
          "The embedding model manifest pins an immutable revision and expected SHA-256 values. Downloads are checked before installation, and every load hashes the installed bytes before fastembed or ONNX Runtime receives them. Load verification uses the bytes already read, avoiding a check of one file followed by loading a changed file.",
          "This relocates trust to the manifest’s anchoring event and the binary carrying it. Matching hashes prove identity with pinned bytes, not that the publisher or model is benign. Successful 384-dimensional normalized output proves function, not safety. The trust document distinguishes cryptographic, procedural, infrastructural, and reputational links.",
          "An installed-file mismatch fails locally without deleting files or initiating replacement downloads. The shell proceeds with embeddings unavailable; library write ownership is unaffected. Both model and database must initialize for semantic capabilities to become available."
        ],
        "example": "An installed tokenizer file changes. The next load refuses before constructing the embedder, preserves the files for diagnosis, and does not reinterpret the change as a routine remote update.",
        "questions": [
          {
            "prompt": "What does a successful hash comparison establish?",
            "answers": [
              {
                "text": "The bytes match the pinned manifest, subject to trust in that manifest and binary.",
                "feedback": "Integrity is relative to the chosen anchor.",
                "correct": true
              },
              {
                "text": "The publisher is proven benign.",
                "feedback": "Hashes do not prove intent or semantic safety.",
                "correct": false
              },
              {
                "text": "Every future model version is automatically approved.",
                "feedback": "New artifacts require a new anchoring decision.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "How should installed-file verification failure behave?",
            "answers": [
              {
                "text": "Refuse model loading locally while preserving files and allowing the shell’s degraded mode.",
                "feedback": "Verification is refusal, not automatic repair or library-mode selection.",
                "correct": true
              },
              {
                "text": "Delete the files and silently fetch new ones.",
                "feedback": "The documented load path neither deletes nor initiates download.",
                "correct": false
              },
              {
                "text": "Accept the changed file if it still yields a vector.",
                "feedback": "Functional output does not substitute for integrity verification.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Embedding trust chain and runtime failure",
            "path": "../architecture/security/embedding_model_trust.md"
          },
          {
            "title": "Model verification implementation",
            "path": "../../crates/xaga-core/src/embeddings/model.rs"
          },
          {
            "title": "Embedding startup UI",
            "path": "../../src/providers/EmbeddingStartupProvider.tsx"
          }
        ]
      },
      {
        "id": "scale-review",
        "level": "Advanced",
        "title": "Review a change at the declared scale",
        "objective": "Combine bounded work, recovery, visibility, and evidence in an engineering review.",
        "paragraphs": [
          "The design target is a multi-million-Entry corpus, around four times expected intake. A loop that is acceptable for a single live edit can become fragment explosion or unbounded memory during bulk import, transition replay, or fresh recovery. Evaluate the bulk path within the target, not just the common edit.",
          "Batching and checkpoints bound retained application work and restart cost; they do not automatically bound query-engine allocations or total process memory. Likewise, a LabelList or bitmap index narrows selection but does not remove the cost of materializing a large result. Separate measured observations from modeled estimates and excluded costs.",
          "A complete change review names source authority, writer rights and lanes, job priority, every commit boundary, recovery detection, native trust boundaries, and verification at the changed layer. Keep visibility narrow and consolidate duplicate logic deliberately. Deferred assumptions are recorded with fact and location; severity and scheduling remain explicit decisions."
        ],
        "example": "For a new bulk state operation, specify candidate batch bounds, pending state before writes, retry behavior after each commit, current-metadata preservation, and tests that interrupt the sequence. A small happy-path run alone cannot verify those properties.",
        "questions": [
          {
            "prompt": "A batch holds at most 1,024 candidate IDs. Which claim follows?",
            "answers": [
              {
                "text": "That candidate list is bounded; total process memory still needs separate evidence.",
                "feedback": "Parsing, query-engine, cache, and other allocations remain separate.",
                "correct": true
              },
              {
                "text": "The whole process has a proven constant memory ceiling.",
                "feedback": "A bound on one structure is not a process-wide bound.",
                "correct": false
              },
              {
                "text": "No interruption tests are needed.",
                "feedback": "Batch size does not establish correct checkpoint ordering.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Which verification best addresses a multi-commit crash story?",
            "answers": [
              {
                "text": "Interrupt between commits, retry, and compare authoritative files with repaired derived state.",
                "feedback": "That checks the partial states the contract actually permits.",
                "correct": true
              },
              {
                "text": "Only check that the final success toast appears.",
                "feedback": "A happy-path toast does not exercise restart behavior.",
                "correct": false
              },
              {
                "text": "Benchmark one Entry and assume linearity at two million.",
                "feedback": "The design-scale document warns against unsupported extrapolation.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Design-scale evidence",
            "path": "../architecture/design_scale.md"
          },
          {
            "title": "Crash consistency",
            "path": "../architecture/crash_consistency.md"
          },
          {
            "title": "Least visibility",
            "path": "../principles/least_visibility_principle.md"
          },
          {
            "title": "Consolidation and DRY",
            "path": "../principles/dry.md"
          },
          {
            "title": "Development checks",
            "path": "../development/release_cycle.md#development-and-local-builds"
          }
        ]
      }
    ]
  }
};
