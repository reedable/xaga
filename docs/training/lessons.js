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
          "Xaga is a Personal Knowledge Management tool: a place to save material, read it, connect it to what you already know, and develop your own insights. Receiving an article before everyone else is not its goal.",
          "When assessing a product idea, ask how it helps that process. Bringing in more articles may be useful, but the number received does not tell you whether the reader understands more. Explain how a proposed feature helps someone read, interpret, connect, or revisit material.",
          "The README commits Xaga to privacy and accessibility. It also states a limit: protecting how articles are fetched and stored does not establish whether they are true. The reader still has to judge their claims."
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
            "prompt": "Xaga loads an article using its security controls. What does that tell us about the article?",
            "answers": [
              {
                "text": "The loading controls were applied, but the reader still needs to judge its claims.",
                "feedback": "Protecting how content is fetched does not establish whether it is true.",
                "correct": true
              },
              {
                "text": "Its arguments have been fact-checked.",
                "feedback": "Network security checks do not fact-check arguments.",
                "correct": false
              },
              {
                "text": "Its popularity proves its reliability.",
                "feedback": "The README warns against treating popularity as evidence of truth.",
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
        "objective": "Explain how Feeds, Topics, and Sites group articles.",
        "paragraphs": [
          "A Feed brings articles into the library. A Topic groups material that belongs together. Both have their own saved records. In the code, they share a type called `ManagedCollection`: each exists independently of the articles it currently contains.",
          "Sites lets you browse articles by website. Xaga gets the website name from each article’s address and groups the articles automatically. The same Site can include articles from different feeds. You do not create a Site or maintain its article list yourself.",
          "The code uses `Collection` for the information needed to browse a Feed, Topic, or Site: its ID, title, and article counts. This does not mean all three are stored in the same way. The organization-model document describes the intended design and notes that some code changes may still be outstanding; check the relevant behavior document and code before claiming a feature is available."
        ],
        "example": "Articles at https://www.example.com/story and https://example.com/other appear under example.com. Xaga treats the leading www. as the same website. An article at https://news.example.com/story appears under news.example.com instead.",
        "questions": [
          {
            "prompt": "In Xaga, Sites lets you browse articles by website. Two different feeds bring in articles from example.com. When you open example.com in Sites, what should you see?",
            "answers": [
              {
                "text": "Articles from example.com, regardless of which feed brought them in.",
                "feedback": "Xaga uses each article’s web address to group it under a Site. The feed it came from does not matter.",
                "correct": true
              },
              {
                "text": "Only the articles from the feed you added first.",
                "feedback": "Both feeds can contribute articles to the same Site. The order in which you added the feeds does not matter.",
                "correct": false
              },
              {
                "text": "No articles until you manually add them to that Site.",
                "feedback": "Xaga groups articles by website automatically. You do not have to add them to a Site yourself.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "You save your first article from a new website. Do you need to create a Site for that website in Xaga?",
            "answers": [
              {
                "text": "No. Xaga automatically includes the website in Sites based on the article’s web address.",
                "feedback": "Sites come from the articles in your library. Unlike a Topic, a Site does not need to be created separately.",
                "correct": true
              },
              {
                "text": "Yes. You must create and name the Site yourself.",
                "feedback": "You do not create Sites manually. Xaga gets the website name from the article’s web address.",
                "correct": false
              },
              {
                "text": "Yes. You must subscribe to a feed from that website first.",
                "feedback": "A Site does not require a feed subscription. An article from the website is enough.",
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
        "objective": "Distinguish read status, bookmark color, and tags.",
        "paragraphs": [
          "An article can be read or unread. This is separate from its tags and bookmark. The unread count beside a Feed, Topic, or Site counts its unread articles; it does not mean the collection has its own reading state.",
          "Bookmarking an article gives it a color: Red, Orange, Yellow, Green, Blue, or Violet. Changing the color keeps it bookmarked. Removing the color removes the bookmark. Xaga does not store an extra yes/no bookmark value alongside that color.",
          "Tags are labels you can add to organize articles. An article can have several tags. Removing a tag from one article leaves the tag available for other articles. Saved includes both bookmarked articles and annotations, so it contains more than bookmarked articles alone."
        ],
        "example": "An unread article can be bookmarked Violet and tagged Research at the same time. Marking it read should not remove either the color or the tag.",
        "questions": [
          {
            "prompt": "A bookmarked article changes from Blue to Violet. What happens?",
            "answers": [
              {
                "text": "It remains bookmarked with Violet as its color.",
                "feedback": "Giving the article another bookmark color keeps it bookmarked.",
                "correct": true
              },
              {
                "text": "It becomes unbookmarked until a flag is set.",
                "feedback": "The color itself records the bookmark; there is no separate yes/no value to switch on.",
                "correct": false
              },
              {
                "text": "The article becomes unread again.",
                "feedback": "Bookmark color and reading state are independent.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "An article has a tag named “Research”. You remove that tag from this article. What should happen?",
            "answers": [
              {
                "text": "The article no longer has the Research tag. The tag remains available for other articles.",
                "feedback": "Removing a tag from one article does not delete the tag or the article. Other articles keep their tags.",
                "correct": true
              },
              {
                "text": "Remove the Research tag from every article.",
                "feedback": "You removed the tag from only this article. Deleting the tag everywhere is a separate action that requires confirmation.",
                "correct": false
              },
              {
                "text": "Remove the article’s bookmark as well.",
                "feedback": "Removing a tag does not change whether the article is bookmarked.",
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
          "When you open an unread article and Xaga marks it as read, it stays in the list you were browsing. Its appearance changes to show that it has been read. Changing or removing a bookmark also leaves the article in the current list, including a Saved list.",
          "Choosing another list or changing a filter asks Xaga for a new set of articles. Keeping the current list stable while you read lets you see what you just opened without losing your place.",
          "Removing a shortcut does not close the list or article you opened through it. Removing a shortcut and deleting the article or collection are different actions; the rule about keeping your place does not specify what happens when content itself is deleted."
        ],
        "example": "Look at your unread articles and open the first one. It stays in the list after being marked read. If you later reopen the unread list, Xaga can show a new list that excludes articles you have read.",
        "questions": [
          {
            "prompt": "You’re looking at your unread articles and open one to read it. Xaga marks it as read. Should it disappear from the list, or stay visible?",
            "answers": [
              {
                "text": "Keep the article in the list, but show that it has been read.",
                "feedback": "The article stays visible until you ask for a fresh list, for example by changing the filter or opening another list.",
                "correct": true
              },
              {
                "text": "Remove the article from the list immediately.",
                "feedback": "Removing it immediately would make you lose your place while reading.",
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
            "prompt": "You open a Topic through a shortcut, then remove that shortcut. What should happen?",
            "answers": [
              {
                "text": "The shortcut disappears, but you can keep reading the Topic’s articles.",
                "feedback": "Removing a shortcut does not close what you opened through it.",
                "correct": true
              },
              {
                "text": "The articles are removed from the Topic.",
                "feedback": "A shortcut does not decide which articles belong to a Topic.",
                "correct": false
              },
              {
                "text": "The open article closes.",
                "feedback": "Removing the shortcut preserves the open list and article.",
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
        "title": "Shortcuts point to saved material",
        "objective": "Explain what a shortcut saves and what happens when its destination is unavailable.",
        "paragraphs": [
          "A shortcut points to an article, Feed, Topic, or Site. The library saves these references centrally in `pins.yaml`, rather than changing the article or collection being pinned. Titles and counts are read from the referenced objects. Sorting shortcuts changes their display order, not the saved sequence.",
          "If Xaga cannot find a shortcut’s destination, it keeps the shortcut and offers Retry and Remove shortcut. It should not say the destination was deleted: a failed read can also make something temporarily unavailable.",
          "A pinned Site with no articles is still a valid destination and shows zero counts. In a read-only library, you can still open shortcuts and retry unavailable ones, but you cannot remove a shortcut because that changes saved data."
        ],
        "example": "A Topic is temporarily unreadable. Keep its shortcut so that it can work again when the Topic becomes readable.",
        "questions": [
          {
            "prompt": "A Topic shortcut is still saved, but Xaga cannot currently read the Topic. What should the interface offer?",
            "answers": [
              {
                "text": "Keep the shortcut, with Retry and Remove shortcut actions.",
                "feedback": "The Topic may become readable again. A failed read is not proof that it was deleted.",
                "correct": true
              },
              {
                "text": "Automatically remove the shortcut.",
                "feedback": "That would discard something the user saved, even if the Topic is only temporarily unavailable.",
                "correct": false
              },
              {
                "text": "Create a new Topic with the same title.",
                "feedback": "A new Topic would not restore the original Topic and its contents.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A Site is in Shortcuts, but no articles from that website remain in the library. What should happen?",
            "answers": [
              {
                "text": "The shortcut still opens the Site, showing no articles and zero counts.",
                "feedback": "A Site can remain a valid destination even when it has no articles.",
                "correct": true
              },
              {
                "text": "The shortcut becomes permanently broken.",
                "feedback": "The website can still be opened as an empty Site.",
                "correct": false
              },
              {
                "text": "Xaga should delete the Site’s saved article list.",
                "feedback": "Sites do not have a separate saved article list; their articles are found from article addresses.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Pin storage rules",
            "path": "../architecture/pins.md"
          },
          {
            "title": "Site shortcuts",
            "path": "../architecture/sites.md#shortcuts"
          },
          {
            "title": "Resolving shortcut destinations",
            "path": "../../src/hooks/useShortcutTargets.ts"
          }
        ]
      },
      {
        "id": "tags",
        "level": "Applied",
        "title": "Tag identity survives a rename",
        "objective": "Distinguish removing a tag from one article, renaming it, merging it, and deleting it.",
        "paragraphs": [
          "Each tag has a permanent ID and a name shown to the user. Renaming the tag keeps that ID. Xaga compares names after removing spaces at either end and converting letters to lowercase. Research and RESEARCH match; straße and strasse do not. Similar-looking names are not necessarily the same tag.",
          "If you rename a tag to a name already in use, Xaga asks separately whether to merge the two tags. A merge uses the destination name. Internally it keeps the ID with more indexed article assignments; if the counts are equal, it keeps the destination ID. The name shown to the user does not decide which ID survives.",
          "Merging loses the distinction between articles that had one tag, the other, or both. The confirmation must explain that this cannot be undone. Deleting a tag removes it from articles without deleting the articles. Renames, merges, and deletions save enough progress to resume after interruption. Their completion uses the article index rather than checking every file: a missed file can later cause an old tag name to appear again as a new tag."
        ],
        "example": "You try to rename Research to Notes, but Notes already exists. Xaga should ask whether to merge them and explain the consequences. It must not silently merge them as part of the rename.",
        "questions": [
          {
            "prompt": "Which two tag names does Xaga treat as the same name?",
            "answers": [
              {
                "text": "Research and RESEARCH.",
                "feedback": "Xaga ignores letter case when comparing names but keeps the display spelling.",
                "correct": true
              },
              {
                "text": "straße and strasse.",
                "feedback": "Converting letters to lowercase does not change ß to ss.",
                "correct": false
              },
              {
                "text": "Any two names that look alike.",
                "feedback": "The comparison uses specified text rules, not visual similarity.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "You rename Research to Notes, but a tag named Notes already exists. What should Xaga do?",
            "answers": [
              {
                "text": "Ask separately whether to merge the tags and explain that their original assignments cannot be restored.",
                "feedback": "Merging changes which tags articles have and cannot be silently treated as a rename.",
                "correct": true
              },
              {
                "text": "Merge the tags immediately without asking.",
                "feedback": "The requested name does not authorize an irreversible merge.",
                "correct": false
              },
              {
                "text": "Delete the articles with the Research tag.",
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
        "title": "Read-only mode and semantic features are separate",
        "objective": "Distinguish permission to change a library from availability of semantic features.",
        "paragraphs": [
          "A read-only library lets you view existing data but refuses changes. Whether semantic features work is a separate question. Semantic search finds material by meaning; Related and relevance ranking also depend on the embedding model and its database. Embeddings are numerical representations of content used by those features.",
          "Embedding startup reports loading, available, or unavailable. While it is loading, the main window waits. If initialization fails, the window opens with the affected features unavailable. Both the model and database must be ready for those features to work.",
          "The app can open a library read-only because another process is writing to it or because its stored version is missing, different, incomplete, or unreadable. Opening it does not automatically upgrade it. Explain what the user can do and why something is disabled, rather than treating all failures as the same problem."
        ],
        "example": "The embedding model fails verification, but the app still has permission to write to the library. Ordinary reading and editing remain available; semantic search and Related do not.",
        "questions": [
          {
            "prompt": "Can semantic search work while a library is read-only?",
            "answers": [
              {
                "text": "Yes, if the existing database and query model are ready.",
                "feedback": "Searching existing data does not require permission to change the library.",
                "correct": true
              },
              {
                "text": "No. Every search needs permission to write to the library.",
                "feedback": "Read-only instances can initialize a model and database for queries.",
                "correct": false
              },
              {
                "text": "Only if opening the library automatically upgrades it.",
                "feedback": "Opening a library does not automatically upgrade it.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "The model fails verification, but Xaga still has permission to change the library. What should remain possible?",
            "answers": [
              {
                "text": "Ordinary library changes, while semantic search and related features are unavailable.",
                "feedback": "Model failure does not remove permission to write to the library.",
                "correct": true
              },
              {
                "text": "Nothing can be changed because model failure makes the library read-only.",
                "feedback": "Library mode depends on whether the app holds its writer, not whether the model is ready.",
                "correct": false
              },
              {
                "text": "Nothing can open until model loading succeeds.",
                "feedback": "An initialization failure allows the main window to open with the affected features unavailable.",
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
        "objective": "Assess how a loading or performance change could let websites track the reader.",
        "paragraphs": [
          "Xaga starts with secure settings. If compatibility requires a less secure option, the user controls that choice and is told its consequences. A website failing to load does not authorize the app to relax security settings silently.",
          "Xaga creates feed-fetching clients for each command and limits the lifetime of article-loading webviews, the embedded browser windows used to load pages. Reusing them could let websites recognize later requests through retained cookies, connections, or other server-provided values. For example, a server can give a client an ETag, a cache validator that the client sends back to check whether content changed. A unique ETag can also identify that client. A proposed speed improvement must account for this privacy cost.",
          "The network design aims to restrict destinations, reveal as little truthful information as possible, and limit resource use. The fetching document also reports cases where WebKit connects to loopback addresses without using the proxy. Loopback addresses reach the user’s own machine. Do not describe intended protections as proven for every request when these gaps remain."
        ],
        "example": "A proposal to reuse loader webviews may improve load time. The review must also identify which website-provided values would survive and how they could let a website recognize later visits.",
        "questions": [
          {
            "prompt": "A proposal would speed up later fetches by keeping server-provided cache validators such as ETags. How should it be reviewed?",
            "answers": [
              {
                "text": "As a performance and privacy trade-off requiring an explicit decision.",
                "feedback": "A website can give a client a unique validator value and recognize it when the client sends that value back later.",
                "correct": true
              },
              {
                "text": "As a speed improvement with no effect on privacy.",
                "feedback": "Keeping website-provided data between requests can let a website recognize the reader.",
                "correct": false
              },
              {
                "text": "As proof that the privacy defaults are defective.",
                "feedback": "The loading cost must be weighed against privacy; it does not justify changing the default silently.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Which security statement is supported by the fetching document?",
            "answers": [
              {
                "text": "The proxy enforces its policy on requests it receives, with documented WebKit bypass gaps.",
                "feedback": "The proxy can check only requests that actually reach it; the documented bypasses remain a limitation.",
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
        "title": "Understand what a performance measurement proves",
        "objective": "Tell measured performance apart from estimates and identify what a measurement leaves out.",
        "paragraphs": [
          "Xaga expects roughly 240–360 thousand articles per year and is designed for around four times that intake: a library containing millions of articles. A small demonstration can show whether an interaction makes sense, but cannot establish performance at that size.",
          "The design-scale document distinguishes actual measurements from estimates. Some timings cover only database queries and exclude moving results to the UI, creating objects, sorting, and drawing the interface. A database index can find matches quickly while returning a very large set of results still takes time and memory.",
          "List virtualization means drawing only the list items currently needed on screen. It does not prevent the app from receiving or storing the full query result in memory. Acceptance criteria should identify the operation, library size, number of matches, and which parts were measured. Record assumptions that have not been tested rather than presenting them as established results."
        ],
        "example": "A query returning a small fraction of 64 thousand database records runs quickly. That does not establish how long the UI takes to display a list drawn from two million matching articles.",
        "questions": [
          {
            "prompt": "Xaga draws only the list items currently needed on screen. What does this limit?",
            "answers": [
              {
                "text": "The items drawn on screen, but not necessarily the query result or its memory use.",
                "feedback": "The app can still receive all matching articles from the database.",
                "correct": true
              },
              {
                "text": "All memory use, regardless of library size.",
                "feedback": "The full query result may still occupy memory.",
                "correct": false
              },
              {
                "text": "The need for database indexes.",
                "feedback": "Finding articles and drawing the visible list are separate work.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A document estimates performance for a million articles using a smaller test. How should that result be presented?",
            "answers": [
              {
                "text": "As an estimate, with its assumptions and the work it leaves out.",
                "feedback": "An estimate is not a measurement of the complete operation.",
                "correct": true
              },
              {
                "text": "As a measured guarantee for users.",
                "feedback": "The larger operation has not been measured.",
                "correct": false
              },
              {
                "text": "As irrelevant because the demonstration library is small.",
                "feedback": "Xaga is designed for libraries containing millions of articles.",
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
        "title": "Review a proposal that changes several workflows",
        "objective": "Review a proposal against Xaga’s existing behavior and explain its consequences.",
        "paragraphs": [
          "Consider a proposal to remove articles from the list as soon as they are read, discard unavailable shortcuts, and merge tags without asking when names match. Each changes an existing rule: keeping your place, retaining a shortcut that may become usable again, or asking before an irreversible merge.",
          "For each change, identify what the user did and what saved data would change. Moving to another view, removing a tag from one article, renaming a tag, and deleting content have different consequences even when their controls look similar.",
          "Present the alternatives and consequences to the person who owns the product decision. If a change is approved, update the lasting documentation with the new behavior. Task stories describe work still to do; they should not be the only record of how the product works."
        ],
        "example": "For an action affecting many articles, ask what happens to their tags, the open list, read-only access, and saved changes if the operation is interrupted. Assess those consequences alongside how the controls look.",
        "questions": [
          {
            "prompt": "Which response best reviews the “cleaner interface” proposal?",
            "answers": [
              {
                "text": "Explain which three existing behaviors would change and present their consequences for a decision.",
                "feedback": "The changes affect what is saved and what the user can keep reading, not just appearance.",
                "correct": true
              },
              {
                "text": "Implement it because fewer items in the list always means simpler UX.",
                "feedback": "Showing fewer things does not by itself justify changing these behaviors.",
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
                "text": "In the lasting documentation, with a task story describing only work still to do.",
                "feedback": "The documentation convention keeps product and engineering rules separate from unfinished tasks.",
                "correct": true
              },
              {
                "text": "Only in an active task forever.",
                "feedback": "Task stories are deleted when finished, so lasting documentation must not rely on them.",
                "correct": false
              },
              {
                "text": "Only in a screenshot.",
                "feedback": "A screenshot does not explain which data changes, what happens on failure, or how the feature behaves.",
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
    "description": "Connect interface behavior to Platform, saved changes, and safe content display.",
    "lessons": [
      {
        "id": "port",
        "level": "Introduction",
        "title": "How the UI uses Platform",
        "objective": "Explain how UI components request work from the Rust backend.",
        "paragraphs": [
          "Xaga’s UI is written in React and TypeScript. Components use the `Platform` interface to request backend operations and listen for events. `TauriPlatform` implements that interface by communicating with the Rust backend and converting the returned data into the objects used by the UI. `main.tsx` creates it and makes it available through `PlatformContext`.",
          "Components should use `Platform` instead of calling native commands directly. This keeps native communication in one implementation. Add a `Platform` method when a component needs an operation; do not add methods only because they might be useful later.",
          "A mock implementation could supply sample data without Rust. Starting the Vite server alone does not provide that implementation or replace native startup calls: `main.tsx` also asks Tauri which window is running. A browser-only capture setup would need to handle those dependencies explicitly."
        ],
        "example": "Follow a bookmark action from a component through `Platform` and `TauriPlatform` to the Rust bookmark command. `Platform` declares the operation; it does not save library data itself.",
        "questions": [
          {
            "prompt": "A React component needs to mark an article as read. How should it request that operation?",
            "answers": [
              {
                "text": "Call the appropriate `Platform` method.",
                "feedback": "`TauriPlatform` handles the native communication behind that interface.",
                "correct": true
              },
              {
                "text": "Add a direct native invoke call inside the component.",
                "feedback": "That would spread native communication outside the platform implementation.",
                "correct": false
              },
              {
                "text": "Write to the library’s Markdown file from the browser.",
                "feedback": "Library writes go through the Rust writer.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Does starting the Vite development server alone let Xaga run without Rust?",
            "answers": [
              {
                "text": "No. Native startup calls and `TauriPlatform` still need replacements.",
                "feedback": "`main.tsx` creates `TauriPlatform` and reads the native window label.",
                "correct": true
              },
              {
                "text": "Yes. A TypeScript interface automatically supplies sample data.",
                "feedback": "An interface declares methods; it does not implement them.",
                "correct": false
              },
              {
                "text": "Yes. The training page already replaces the app’s backend.",
                "feedback": "The training page is separate from the React app.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Platform interface",
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
        "title": "Article summaries, loaded articles, and collections",
        "objective": "Recognize Xaga’s UI types and use their fields according to their meaning.",
        "paragraphs": [
          "The `Entity` type supplies `id` and `title`. `ManagedEntity` adds fields such as `createdAt` for objects with their own saved record. `Collection` adds article counts and a `__kind` value of feed, topic, or site. Feed and Topic have saved membership lists; Site is calculated from article addresses.",
          "Entry describes an article’s summary and state. Article adds its loaded body and annotations while keeping `__kind`: entry. Annotation describes a highlighted passage and its optional comment; its content field is that comment, not a separate Note object. These distinctions determine which article or annotation an action affects.",
          "Type guards are functions that check whether an object has the expected fields and `__kind` value. The docs call `__kind` a brand: it distinguishes roles that can have similar fields. A brand alone is not enough. Article counts also do not turn an Entry into a Collection. Use the supplied total counts rather than `members.length`, because the loaded member list can be incomplete."
        ],
        "example": "A Site card can use `id`, `title`, `entryCount`, and `unreadCount`. Do not invent a `createdAt` date or a `members` list just to make it look like a Topic object.",
        "questions": [
          {
            "prompt": "A Feed’s loaded `members` list has ten articles, but `entryCount` reports more. Which value should its article count show?",
            "answers": [
              {
                "text": "`entryCount`.",
                "feedback": "The loaded `members` list can be incomplete; the supplied summary count covers the collection.",
                "correct": true
              },
              {
                "text": "`members.length`.",
                "feedback": "That would count only the members loaded into this object.",
                "correct": false
              },
              {
                "text": "The average of `entryCount` and `members.length`.",
                "feedback": "These describe different things. Averaging them does not produce a valid count.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "What distinguishes an Article from an Entry summary in the UI types?",
            "answers": [
              {
                "text": "It keeps the entry kind and adds content and annotations.",
                "feedback": "An Article is an Entry with its body and annotations loaded.",
                "correct": true
              },
              {
                "text": "It replaces the entry kind with an article kind.",
                "feedback": "The Article type still has `__kind`: entry.",
                "correct": false
              },
              {
                "text": "It has a `title`.",
                "feedback": "Other `Entity` objects have titles too.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Domain types and type-checking functions",
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
          "Xaga’s base interface must work without animation. Add animation only inside the CSS media query `prefers-reduced-motion: no-preference`. A browser that does not support that query, or a user who requests reduced motion, gets the complete static interface.",
          "The reload icon follows this rule. Its base styles set animation to none, and the no-preference query adds spinning while reload is in progress. The control’s `title` and screen-reader status explain that work is running even when the icon does not spin.",
          "The same rule applies to transitions and third-party components. The Inspector drawer and tab indicator must respect the motion preference. If a component cannot do that, explain its limitation and the alternatives rather than relying on its default animation."
        ],
        "example": "A reload control can show “Reload in progress...” without animation. A spinner adds another cue but cannot be the only indication that the operation is running.",
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
                "text": "Status text that works without animation, including screen-reader feedback.",
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
        "title": "The list and the selected article are different state",
        "objective": "Keep the current list and the selected article separate when handling navigation.",
        "paragraphs": [
          "The router tracks the list being browsed separately from the article or annotation selected in it. The code calls the list’s defining collection or article its scope. For an article shortcut, `articleScopeId` identifies the article whose list is open; `entryId` and `annotationId` identify what the reader is showing.",
          "Reading-state and bookmark-color events update articles already shown without changing which articles are in the list. Removing a shortcut also leaves the open list in place. Choosing another collection or changing a filter starts a new query. Reloading the list for every event would lose these distinctions.",
          "A query result must still match the view that requested it. If the user changes the selected tags while a query is running, its old result must not replace the new list. Selected tag IDs belong to navigation history, so Back and Forward restore them; they are not saved as library settings."
        ],
        "example": "Open a pinned article’s list, then choose one of its annotations. The reader opens the article at that passage. The list still shows the pinned article and its annotations.",
        "questions": [
          {
            "prompt": "Which action should leave the same articles in the list you are browsing?",
            "answers": [
              {
                "text": "Changing the bookmark color of an article already in that list.",
                "feedback": "Its bookmark changes, but the current list is preserved.",
                "correct": true
              },
              {
                "text": "Choosing another Feed or Topic.",
                "feedback": "Choosing another collection asks for a new list.",
                "correct": false
              },
              {
                "text": "Changing which tags are selected.",
                "feedback": "Changing selected tags asks for a new matching set of articles.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "You select different tags while a query is running. Its old result arrives after the new result. What should happen?",
            "answers": [
              {
                "text": "Ignore the old result because it belongs to the previous selection.",
                "feedback": "The displayed list must match the user’s latest selection.",
                "correct": true
              },
              {
                "text": "Display the old result because it arrived last.",
                "feedback": "Response order does not determine which selection is current.",
                "correct": false
              },
              {
                "text": "Save both selections in `library.yaml`.",
                "feedback": "These selections belong to navigation history, not library-version metadata.",
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
        "objective": "Distinguish what is being inspected from where the Inspector appears.",
        "paragraphs": [
          "The shared Inspector shows Related and Topics for Articles and Annotations, and Tags for Articles. Selecting an article or annotation does not open a closed drawer. Inspect is a separate action. The Inspector must know whether you are inspecting the article or a particular annotation.",
          "Article inspection uses a 280-pixel side panel when the reader’s container is at least 700 CSS pixels wide, and a drawer when it is narrower. Measure the reader’s available space, not the full browser window. Annotation inspection always uses the drawer, even on wide layouts.",
          "Opening the Inspector preserves your reading position. Its drawer has an accessible name, a Close action, Escape dismissal, and keyboard focus that stays inside until it closes. Focus then returns to the opening control, or the reader if that control is gone. Article and Annotation tabs are remembered separately for the app session. Switching tabs keeps their components mounted rather than discarding their local state."
        ],
        "example": "A wide browser can still have a narrow reader because surrounding panes consume space. Use the reader’s width to decide whether Article inspection fits beside it.",
        "questions": [
          {
            "prompt": "You inspect an Annotation while the reader is wide enough for an Article side panel. Where should Annotation inspection open?",
            "answers": [
              {
                "text": "In the Inspector drawer.",
                "feedback": "Annotation inspection always uses the drawer.",
                "correct": true
              },
              {
                "text": "In the Article side panel, replacing its Article.",
                "feedback": "The Article panel continues to inspect the Article; the drawer inspects the Annotation.",
                "correct": false
              },
              {
                "text": "In a new browser window.",
                "feedback": "That is not the documented interface.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "When should Article inspection use the side panel rather than a drawer?",
            "answers": [
              {
                "text": "When the reader container is at least 700 CSS pixels wide.",
                "feedback": "Measure the space available to the reader, after other panes take their space.",
                "correct": true
              },
              {
                "text": "Whenever the browser window is at least 700 pixels wide.",
                "feedback": "The reader can be narrower than the browser window.",
                "correct": false
              },
              {
                "text": "Whenever the article has a wide image.",
                "feedback": "Article image width does not determine the panel layout.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Inspector behavior",
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
        "title": "Show saved changes without waiting for the index",
        "objective": "Show successful Topic and tag changes even while the database is catching up.",
        "paragraphs": [
          "The `TopicsProvider` remembers successful additions to and removals from Topics while a library is open. It combines those changes with the database’s reverse index, which answers which Topics contain an article or annotation. This index can lag behind the file write, so the remembered changes keep the UI current.",
          "Before the first Topics list loads, show Loading Topics…. An unanswered request does not mean there are no Topics. A failed save must not appear as a successful membership change. If the inspected article or annotation changes, an earlier request must not overwrite the new selection’s display.",
          "The Article Tags view reads assignments from files. After a successful save, `EntryTagChange` reports the article ID, tag ID, and whether the tag was added or removed. Tag buttons expose `aria-pressed` so assistive technology can identify selected tags. Changes save immediately; there is no Save button. In read-only mode, assignments remain visible but cannot be changed."
        ],
        "example": "Adding an article to a Topic succeeds. Close and reopen the Topic picker before the database catches up. `TopicsProvider` should retain the successful change, so the Topic remains selected.",
        "questions": [
          {
            "prompt": "Adding an article to a Topic fails to save. What should the Topic picker show?",
            "answers": [
              {
                "text": "Keep the previous selection and report the failure.",
                "feedback": "The UI should not show the change as saved when it failed.",
                "correct": true
              },
              {
                "text": "Show the Topic as selected as if the save succeeded.",
                "feedback": "That would misrepresent what was saved.",
                "correct": false
              },
              {
                "text": "Remove all Topics from the picker.",
                "feedback": "One failed save does not mean the Topics are unavailable.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "The Topics list has not finished loading for the first time. What should the picker show?",
            "answers": [
              {
                "text": "Loading Topics….",
                "feedback": "A pending response does not mean there are no Topics.",
                "correct": true
              },
              {
                "text": "Create a New Topic, implying no Topics exist.",
                "feedback": "That would confuse data that has not loaded with an empty list.",
                "correct": false
              },
              {
                "text": "The list from the previously opened library.",
                "feedback": "Data from another library must not appear as this library’s Topics.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Showing saved Topic changes",
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
        "title": "Browse articles with several tags",
        "objective": "Explain which articles appear when tags are selected and preserve those selections through updates.",
        "paragraphs": [
          "Selecting several Tags shows articles with every selected tag. This is called intersection. Selecting Research and History means both, not either one. With no tags selected, Tags browsing shows all Entries under the current filters; it does not include the Annotations shown by the separate All items shortcut.",
          "When tags are selected, Unread is hidden and a previously selected Unread filter is temporarily treated as All. Saved, All, and text filtering remain available. Clearing the selected tags restores the Unread control and the previous filter. The sidebar and narrow-layout header use the same selected tag IDs.",
          "Renaming a tag changes its label without changing its selected ID or refreshing the article list. If a selected tag is missing from updated registry data, keep it selected as “Unavailable tag” and let the user deselect it. Silently removing it could show articles the user did not ask for. Explicitly deleting a tag removes its selection but keeps the current list until the next refresh or navigation."
        ],
        "example": "Select Research and History to see articles with both tags. Renaming Research should update the label without dropping that tag from the selection.",
        "questions": [
          {
            "prompt": "You select Research and History in Tags. Which articles should appear?",
            "answers": [
              {
                "text": "Articles with both tags.",
                "feedback": "Selecting several tags means every selected tag must be present.",
                "correct": true
              },
              {
                "text": "Articles with either tag.",
                "feedback": "That would show articles missing one of the selected tags.",
                "correct": false
              },
              {
                "text": "Annotations belonging to either Topic.",
                "feedback": "Tags browsing covers Entries, and Tags are different from Topics.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A selected tag is missing from refreshed tag data. What should the UI do?",
            "answers": [
              {
                "text": "Keep it selected as Unavailable tag and allow the user to deselect it.",
                "feedback": "Silently dropping it would change which articles match the user’s selection.",
                "correct": true
              },
              {
                "text": "Display its UUID in place of its name.",
                "feedback": "Internal IDs should not become user-facing tag names.",
                "correct": false
              },
              {
                "text": "Clear all selected tags.",
                "feedback": "That would change the user’s selection without asking.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Tags browsing behavior",
            "path": "../architecture/list_view.md#tag-browsing"
          },
          {
            "title": "Article queries and tag storage",
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
        "title": "Check saved content before displaying it",
        "objective": "Explain why saved content is checked again before display and what those checks do.",
        "paragraphs": [
          "Treat saved content as untrusted when displaying it, even if it was checked before saving. Files can be edited externally, imported, or written by an older version. The display code must apply its own checks rather than assume saved markup is safe.",
          "`markdownPipeline` converts Markdown and embedded HTML to HTML, then uses DOMPurify to remove unsafe markup. This removal is called sanitization. `buildEntryHtml` prepares the reader document, and `HtmlReader` displays it in an iframe. The iframe limits what that document can do; Content Security Policy (CSP) separately restricts scripts and resource loading.",
          "Checking markup does not prove that network requests are isolated. The security docs record cases where WebKit reaches the user’s own machine without going through the configured proxy, alongside separate CSP tests. A passing sanitizer test establishes only the behavior tested, not every protection of the packaged application."
        ],
        "example": "An externally edited article contains HTML that runs code when clicked. Being stored in the library does not make it safe; the display code must still remove unsafe markup.",
        "questions": [
          {
            "prompt": "Why remove unsafe markup again when displaying a saved article?",
            "answers": [
              {
                "text": "The saved file may have changed, and earlier checks do not guarantee it is safe to display now.",
                "feedback": "Read-time checks apply to all saved content, regardless of where it came from.",
                "correct": true
              },
              {
                "text": "Only because it came from a remote feed.",
                "feedback": "Imported, locally edited, and user-authored saved content need the same checks.",
                "correct": false
              },
              {
                "text": "To prove that the article’s claims are true.",
                "feedback": "Removing unsafe markup does not fact-check content.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A DOM test proves an unsafe HTML attribute is removed. What does it establish?",
            "answers": [
              {
                "text": "Only that the tested HTML attribute is removed, not that all native network requests are blocked.",
                "feedback": "Native WebKit networking and other protections need separate tests.",
                "correct": true
              },
              {
                "text": "That WebKit can never reach loopback.",
                "feedback": "The native docs record bypass observations.",
                "correct": false
              },
              {
                "text": "That the model’s installed bytes are verified.",
                "feedback": "Model files are verified separately in the backend; this HTML test says nothing about them.",
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
        "title": "Explain a loading failure without losing the article",
        "objective": "Keep error messages and loading indicators associated with the correct article-loading attempt.",
        "paragraphs": [
          "Each article-loading attempt has a diagnostic ID connecting its frontend and Rust records. Details opens the matching record. Copy diagnostics copies its JSON as plain text. Keep the normal error message understandable; do not replace it with technical data or display diagnostic text as HTML.",
          "The error-notification code queues accepted notifications. A later warning must not replace the current notification or redirect its Details link to another attempt. Notifications with diagnostics remain until dismissed. Opening or closing Details does not dismiss them. Previously saved article content stays visible during a reload and after a reload failure.",
          "Navigating to another article cancels the frontend attempt. Its late completion must not change the next article’s loading indicator. Diagnostic history has size limits, so missing events do not prove an operation never happened. “Image capture enabled” records a preference; it does not establish that a download was attempted or completed."
        ],
        "example": "Article A is reloading when the user opens Article B. If A’s old request finishes later, it must not clear B’s loading indicator or replace B’s content.",
        "questions": [
          {
            "prompt": "A warning about a website icon arrives while an error notification with Details is visible. What should happen?",
            "answers": [
              {
                "text": "Queue the warning without replacing the current notification or its Details link.",
                "feedback": "Each notification must remain connected to its own loading attempt.",
                "correct": true
              },
              {
                "text": "Replace the current notification with the latest message.",
                "feedback": "That could remove access to the error the user is investigating.",
                "correct": false
              },
              {
                "text": "Display the warning’s technical data as HTML.",
                "feedback": "Diagnostic data must be displayed and copied as plain text.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Diagnostics say image capture was enabled. What can you conclude?",
            "answers": [
              {
                "text": "Capture was allowed, but the record does not establish that a download started or finished.",
                "feedback": "Separate diagnostic records report whether capture was allowed, attempted, and completed.",
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
            "title": "Article-loading diagnostics",
            "path": "../architecture/article_loading_diagnostics.md"
          },
          {
            "title": "Frontend loading-attempt tracking",
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
        "title": "Review the behavior a screenshot cannot show",
        "objective": "Review behavior when requests overlap or fail, as well as when they succeed.",
        "paragraphs": [
          "When reviewing a Xaga interface change, identify the article or annotation affected, the list being shown, what has actually been saved, any unfinished requests, and keyboard focus. A screenshot can look right while a late query overwrites a new selection or opening the Inspector reloads the reader.",
          "Use repeatable examples for data that is still loading, genuinely empty, unavailable, read-only, or failed. These states need different explanations and controls. An empty list is not an adequate replacement for all of them.",
          "Choose checks that exercise what changed: component tests for inspected objects and focus, navigation tests for old query results, and native WebKit tests for engine-specific behavior. Keep helpers local until another file needs them. Use names that say what an operation does, following the project’s vocabulary conventions."
        ],
        "example": "For an Inspector refactor, verify Article and Annotation targets, narrow and wide reader containers, keyboard dismissal and focus return, reduced motion, failed writes, and reader-position preservation.",
        "questions": [
          {
            "prompt": "Which test could reveal a wrong Inspector selection that a screenshot misses?",
            "answers": [
              {
                "text": "Switch to another inspected article or annotation before the first lookup finishes.",
                "feedback": "The earlier result must not overwrite the newly inspected object.",
                "correct": true
              },
              {
                "text": "Display one fully loaded article and stop there.",
                "feedback": "That does not test what happens when requests overlap.",
                "correct": false
              },
              {
                "text": "Replace every empty state with a spinner.",
                "feedback": "That hides the difference between loading and genuinely empty data.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A helper function is used only within one file. Should it be exported?",
            "answers": [
              {
                "text": "Keep it local until another file actually needs it.",
                "feedback": "This avoids creating an unnecessary dependency other files can rely on.",
                "correct": true
              },
              {
                "text": "Export it because another file might need it someday.",
                "feedback": "A hypothetical caller does not justify a larger public interface.",
                "correct": false
              },
              {
                "text": "Add an `index.ts` file that re-exports every helper.",
                "feedback": "The project rejects broad re-export files that expose more than callers need.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Inspector behavior",
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
            "title": "Inspector interaction tests",
            "path": "../../src/components/content/EntityView.inspector.test.tsx"
          }
        ]
      }
    ]
  },
  "engineering": {
    "title": "Architect / Backend engineer",
    "description": "Understand how the backend saves data, schedules work, recovers from interruption, and checks untrusted input.",
    "lessons": [
      {
        "id": "layers",
        "level": "Introduction",
        "title": "Separate file operations from command coordination",
        "objective": "Identify which work belongs in core, a Tauri command handler, and the CLI.",
        "paragraphs": [
          "`xaga-core` implements logic and file operations. Tauri command handlers coordinate those operations with queued database work. The CLI handles command-line input and output, database management, and explicit migrations. Put code according to its responsibility, not according to which caller first needed it.",
          "The bookmark commands get the `LibraryWriter`, wait for the color to be saved, and then queue `SyncState` with the saved color. The handler knows what the user did and passes that information along. Core’s file operations do not choose embedding jobs.",
          "Keep progress printing and other presentation in the CLI. If shared logic needs to report progress, define that interaction explicitly. Moving console output into core merely because several callers need the same operation would mix responsibilities."
        ],
        "example": "For a new state change, identify the core file operation, the command handler that waits for it and queues database work, and any separate CLI output.",
        "questions": [
          {
            "prompt": "A bookmark command has saved the article’s color. Where should it queue `SyncState`?",
            "answers": [
              {
                "text": "In the command handler, after the writer succeeds.",
                "feedback": "The handler coordinates saving the file and updating its database copy.",
                "correct": true
              },
              {
                "text": "Inside core’s file-patching function.",
                "feedback": "Core’s file operations do not manage the embedding queue.",
                "correct": false
              },
              {
                "text": "In a file watcher that infers what changed.",
                "feedback": "The command already knows the action and its saved result.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A CLI command needs validation and progress printing. Which part belongs in core?",
            "answers": [
              {
                "text": "Validation, with printing left to the CLI.",
                "feedback": "Core can share the logic without taking responsibility for console output.",
                "correct": true
              },
              {
                "text": "Console formatting because the CLI first needed the operation.",
                "feedback": "The first caller does not decide which layer owns each responsibility.",
                "correct": false
              },
              {
                "text": "Argument parsing and exit-code decisions.",
                "feedback": "Those belong to the command-line interface.",
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
        "objective": "Identify which file owns a value before changing it or rebuilding its database copy.",
        "paragraphs": [
          "Entry Markdown files are the source of truth for content, read state, color, tags, and annotations. Feed Atom files store feed membership. Topic Markdown stores its ordered list of typed member references. `pins.yaml` stores shortcuts, `tags.yaml` stores tag IDs and pending changes, and `library.yaml` stores completed and pending library versions.",
          "LanceDB is the database used to query this data. It is a derived cache: its columns must be rebuildable from the files. The documented rule allows two producers of database updates: live command operations and recovery. An unrelated third write path would violate that rule.",
          "A Topic file answers “what belongs to this Topic?” Its derived reverse index answers “which Topics contain this article or annotation?” Replacing the file’s member list with the reverse index would lose ordering, member types, and annotation references."
        ],
        "example": "Changing Topic membership need not change an article’s content checksum. Recovery must still rebuild the Topic-related database data instead of using that checksum to skip all work.",
        "questions": [
          {
            "prompt": "Which saved data determines a Topic’s members and their order?",
            "answers": [
              {
                "text": "The member references in its Topic Markdown file.",
                "feedback": "The reverse database index supports lookup in the opposite direction.",
                "correct": true
              },
              {
                "text": "Only the topics column on each member’s database record.",
                "feedback": "That column cannot replace the Topic’s ordered, typed member references.",
                "correct": false
              },
              {
                "text": "Whatever the last UI component loaded.",
                "feedback": "The UI displays membership; it does not own the saved list.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "An article’s content checksum has not changed. Why might recovery still need to update the database?",
            "answers": [
              {
                "text": "Reading state, color, or Topic membership can change without changing the article content.",
                "feedback": "Recovery needs checks that detect these changes separately.",
                "correct": true
              },
              {
                "text": "Checksums are never useful.",
                "feedback": "A checksum can detect changes to the content it covers.",
                "correct": false
              },
              {
                "text": "After startup, the database replaces the files as the source of truth.",
                "feedback": "The database remains a copy that can be rebuilt from files.",
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
        "title": "Coordinate writers across processes and within a process",
        "objective": "Distinguish the library lock, per-file write queues, and the lock for shared settings.",
        "paragraphs": [
          "An exclusive advisory flock on `xaga.lock` allows only one cooperating process to write to a library. flock is an operating-system file lock, tied to an open handle. The OS releases it when the handle closes, including on process exit. The lock file remaining on disk does not mean a lock is still held. In the app, having a `LibraryWriter` means read-write mode; having no writer means read-only mode.",
          "`LibraryWriter` also prevents conflicting writes inside that process. Each file has a lane: a queue that runs the entire read-modify-write sequence one operation at a time. Different files can be written concurrently. Atomic rename prevents readers from seeing a half-written file, but alone cannot prevent two writers from reading the same old content and losing one another’s edits.",
          "The account lock on `settings.json` protects settings and model downloads shared across libraries. Operations in the same process share the lock rather than acquiring competing handles. A settings replacement transfers the lock to the new file without a gap. The code also detects settings changed since the caller read them, avoiding a silent overwrite of newer values."
        ],
        "example": "Two Annotation edits on one Article use the same per-file queue even though the process already holds the library lock. A model download also needs the account lock because model files are shared across libraries.",
        "questions": [
          {
            "prompt": "Why does a process holding the library lock still need a separate write queue for each file?",
            "answers": [
              {
                "text": "Its own concurrent operations could otherwise read the same old file and overwrite one another’s edits.",
                "feedback": "The process lock excludes other processes, not overlapping operations inside its holder.",
                "correct": true
              },
              {
                "text": "Atomic rename lets readers see half-written files.",
                "feedback": "Atomic replacement prevents that; lost updates are a different problem.",
                "correct": false
              },
              {
                "text": "Every file needs a separate process.",
                "feedback": "A queue can serialize writes to one file within the same process.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Two instances open different libraries but both update `settings.json`. What coordinates their writes?",
            "answers": [
              {
                "text": "The account lock on `settings.json`.",
                "feedback": "Settings are shared across libraries for the same machine and user.",
                "correct": true
              },
              {
                "text": "Only the first library’s `xaga.lock`.",
                "feedback": "That lock protects its library, not shared settings.",
                "correct": false
              },
              {
                "text": "Nothing, because the library paths differ.",
                "feedback": "The instances still share settings and model resources.",
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
        "title": "Choose priority by who needs the result and when",
        "objective": "Assign queue priority by urgency and preserve each queue’s shutdown rules.",
        "paragraphs": [
          "An embedding `Job` describes what to do; its `Tier` describes when to do it relative to other work. Control, Interactive, Regular, and Maintenance are serviced in that order. The same job can use different tiers when its urgency differs. Expensive work is not automatically low priority.",
          "A feed command can queue the database update needed to show new articles as Interactive, then queue their numerical content representations as Regular. Those representations are called embedding vectors. Recovery can schedule vector creation as Maintenance because nobody is waiting on a newly requested result.",
          "`LibraryWriter` has separate Interactive and Regular tiers. During shutdown, its queued Interactive writes must finish because they can contain unsaved user input. Queued Regular work may be dropped, so only content that can be obtained again belongs there. The embedding service has a different rule: queued derived database work may be dropped and rebuilt by recovery."
        ],
        "example": "A large operation requested by the user can be urgent; a small background repair can wait. `Shutdown` also needs immediate attention even when finishing it is expensive.",
        "questions": [
          {
            "prompt": "What should determine an embedding job’s tier?",
            "answers": [
              {
                "text": "Who needs the result and how urgently they need it.",
                "feedback": "The cost of the work is a separate consideration.",
                "correct": true
              },
              {
                "text": "Whether the action runs the model.",
                "feedback": "The same kind of action can have different urgency.",
                "correct": false
              },
              {
                "text": "Always putting large batches in Maintenance.",
                "feedback": "A user can be waiting for a result from a large operation.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Which work is safe to put in `LibraryWriter`’s Regular queue?",
            "answers": [
              {
                "text": "Work that can be recreated if its queued write is dropped during shutdown.",
                "feedback": "Regular backlog may be dropped; unrecoverable user input must not be placed there.",
                "correct": true
              },
              {
                "text": "An unsaved Annotation because it is small.",
                "feedback": "A small amount of user-authored content can still be irreplaceable.",
                "correct": false
              },
              {
                "text": "Any write, because recovery can recreate lost user input.",
                "feedback": "Recovery works from saved files; it cannot recreate input that never reached them.",
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
        "title": "Plan for interruption between writes",
        "objective": "Describe what survives interruption between writes and how recovery detects unfinished work.",
        "paragraphs": [
          "Graceful shutdown and a forced process termination are different cases. The embedding service handles `Shutdown` between iterations of its processing loop. That does not make a sequence of writes safe from a crash: forced termination can occur between any two commits. Recovery must repair unfinished database work using saved files.",
          "Feed persistence saves article files before the feed’s membership file. Image bytes are saved before Markdown that refers to them. These orders determine what can remain after interruption: an unused image can be cleaned up later, whereas a saved reference to missing bytes would be unusable.",
          "Recovery checks read state and color separately from content checksums. When a queued Annotation embedding finishes, it must check that the database record still exists and matches its input before updating the vector. It must not restore a deleted Annotation or overwrite newer metadata. Every possible interruption needs a check that can detect what remains unfinished."
        ],
        "example": "The process stops after saving a bookmark color to Markdown but before `SyncState` runs. Recovery reads the saved color and repairs the database without recalculating the embedding of unchanged article text.",
        "questions": [
          {
            "prompt": "The app is forcibly terminated after saving an article’s color but before updating LanceDB. What should happen on recovery?",
            "answers": [
              {
                "text": "Read the saved color and repair the database record.",
                "feedback": "The file survived, and recovery checks color separately from article content.",
                "correct": true
              },
              {
                "text": "Replace the file’s color with the old database value.",
                "feedback": "That would make the derived database override the source file.",
                "correct": false
              },
              {
                "text": "Assume no partial state is possible because both steps ran in one service-loop iteration.",
                "feedback": "Forced termination can interrupt the operation between writes.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "An Annotation is deleted while an older embedding job for it is running. What must the job avoid when it finishes?",
            "answers": [
              {
                "text": "Recreating the deleted record or writing older metadata back.",
                "feedback": "Completion must check that the record still exists and matches the job’s input.",
                "correct": true
              },
              {
                "text": "Checking whether the current record still exists.",
                "feedback": "That check is needed to prevent recreating deleted data.",
                "correct": false
              },
              {
                "text": "Updating only the vector on an existing, matching pending record.",
                "feedback": "That is the constrained update the operation is allowed to make.",
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
        "title": "Save tag-operation progress outside the database cache",
        "objective": "Explain how tag operations record unfinished work and what their completion does not prove.",
        "paragraphs": [
          "Tag IDs, names, and unfinished renames, merges, or deletions are saved in `tags.yaml`. Article files store readable tag names. LanceDB stores the corresponding tag UUIDs. A rename keeps the same ID while updating names in the affected files.",
          "The operation saves a batch’s pending work before changing files and advances its checkpoint only after that work succeeds. A checkpoint records how far the operation has progressed. Article changes use their own writer lanes; registry changes use the tags lane. Revision checks reject an outdated worker’s checkpoint. Recovery resumes unfinished operations before discovering ordinary tags from article files.",
          "These operations finish after processing indexed candidates and saved pending work. They do not inspect every source file to prove none were missed. A file absent from the index can retain an old name, which later discovery can create as a separate tag. Names are reserved while an operation is active to prevent conflicts; completing it releases those reservations."
        ],
        "example": "Before deleting a tag named Research, save its name reservation and pending batch of article IDs. Then remove the tag from files and the database. Saved pending IDs let recovery resume the work even after rebuilding the database.",
        "questions": [
          {
            "prompt": "When should a tag operation save the file IDs in its next pending batch?",
            "answers": [
              {
                "text": "Before changing those files.",
                "feedback": "If the process stops during file changes, the batch must already be saved for retry.",
                "correct": true
              },
              {
                "text": "Only after the entire library is processed.",
                "feedback": "That would leave no saved batch to resume after interruption.",
                "correct": false
              },
              {
                "text": "Only in LanceDB.",
                "feedback": "The operation must remain resumable even when the database cache is rebuilt.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "A tag rename finishes, but an article missing from the index still has the old tag name. What can happen later?",
            "answers": [
              {
                "text": "Discovery can create a separate tag with that old name.",
                "feedback": "Completion does not scan every file or keep the old name reserved permanently.",
                "correct": true
              },
              {
                "text": "The rename’s completion proves that every article file was checked.",
                "feedback": "Completion covers indexed candidates and pending work, not a complete source scan.",
                "correct": false
              },
              {
                "text": "The old name remains permanently blocked.",
                "feedback": "Completion releases the name reservation.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Tag storage and unfinished operations",
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
          "Each library stores its completed application major.minor version in `library.yaml`. Patch versions, prerelease labels, and build suffixes do not create separate library formats. Updating the app does not relabel libraries. A missing manifest means the version is unknown, not that the library must use a particular older format.",
          "Before 1.0, the CLI performs explicit conversion while holding the library lock. It records the intended target before changing files or database data and leaves the last completed version in place. Only successful conversion and database reconciliation allow it to mark the target complete. An interrupted upgrade must retry the same target.",
          "The app reads this metadata before starting a writer or embedding service. Missing, different, incomplete, or unreadable versions open read-only without replacing the original manifest. That startup path does not create a missing cache or run recovery. While the app remains at major version 0, every storage-model change requires a minor-version increment."
        ],
        "example": "A crash midway through conversion leaves completed version A and pending target B. The next upgrade retries B; normal opening must not declare B complete merely because some files already changed.",
        "questions": [
          {
            "prompt": "When may an upgrade mark the target library version as complete?",
            "answers": [
              {
                "text": "After both file conversion and database reconciliation succeed.",
                "feedback": "The completed version must describe work that actually finished.",
                "correct": true
              },
              {
                "text": "As soon as the user selects the target version.",
                "feedback": "The target is saved as pending while work remains.",
                "correct": false
              },
              {
                "text": "Whenever a newer application opens the library.",
                "feedback": "Normal opening neither migrates nor relabels the library.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "The app opens a library with no version manifest. What should startup do?",
            "answers": [
              {
                "text": "Open read-only without creating version metadata or guessing an older format.",
                "feedback": "An explicit upgrade is needed to establish the library version.",
                "correct": true
              },
              {
                "text": "Assume the oldest supported format and save that version immediately.",
                "feedback": "A missing file does not establish which format the library uses.",
                "correct": false
              },
              {
                "text": "Use the version of the library opened before this one.",
                "feedback": "Each library has its own version metadata.",
                "correct": false
              }
            ]
          }
        ],
        "references": [
          {
            "title": "Library version rules",
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
        "title": "Check network destinations and native command arguments",
        "objective": "Identify what untrusted callers can make a network request or native command do.",
        "paragraphs": [
          "The network proxy resolves a destination’s hostname, checks every returned IP address, and connects to a validated address without resolving it again. It refuses the destination if any returned address is disallowed. Requests for redirects and page resources receive the same checks when they pass through the proxy. Native WebKit tests have demonstrated loopback connections that bypass it, so proxy checks alone do not prove all connections are contained.",
          "Checking network destinations does not make downloaded HTML safe. Checking HTML before saving does not remove the need to check it again before display. Parsing limits, content sanitization, rendering restrictions, and model verification each address different risks. Identify what an input is allowed to cause at each step.",
          "A registered Tauri command can be called through the invoke API. `command_registry.json` supplies command classifications and generated registration and permissions. Window checks and library-write checks remain separate. For user-selected file paths, keep the native file dialog and file operation together in Rust. Accepting an arbitrary path from JavaScript would let the caller choose which file the command accesses."
        ],
        "example": "An import menu opens its file dialog in Rust and calls a native operation directly. The command for opening a stored PDF calculates its library path in Rust from the source URL; JavaScript cannot choose an arbitrary disk path.",
        "questions": [
          {
            "prompt": "DNS returns both public and private IP addresses for one hostname. Why does the proxy refuse the destination?",
            "answers": [
              {
                "text": "Every address must be allowed; a public address must not hide a disallowed one.",
                "feedback": "The proxy checks the whole result before connecting to a validated address.",
                "correct": true
              },
              {
                "text": "Only the first address matters.",
                "feedback": "Another returned address could lead to a disallowed destination.",
                "correct": false
              },
              {
                "text": "Refusing the address proves that all page HTML is safe.",
                "feedback": "Destination checks do not check the page’s markup.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "Where should an import operation handle the path selected in a file dialog?",
            "answers": [
              {
                "text": "In Rust, keeping the dialog and the authorized file operation together.",
                "feedback": "The webview should not be able to supply an arbitrary filesystem path.",
                "correct": true
              },
              {
                "text": "In a webview-callable command that reads any path JavaScript passes.",
                "feedback": "That would let the caller choose files outside the intended operation.",
                "correct": false
              },
              {
                "text": "Only in CSS.",
                "feedback": "CSS cannot carry out or authorize native file access.",
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
            "title": "Checking saved content before display",
            "path": "../architecture/security/handling_persisted_content.md"
          }
        ]
      },
      {
        "id": "model",
        "level": "Advanced",
        "title": "Matching model bytes does not prove the model is harmless",
        "objective": "Explain what model checksums establish, what they do not, and how verification failure behaves.",
        "paragraphs": [
          "The embedding-model manifest specifies an exact revision and expected SHA-256 hashes. A hash is a digest used to check that file bytes match the expected content. Xaga verifies downloads before installation and installed files before passing their bytes to fastembed or ONNX Runtime. It uses those same checked bytes, rather than checking a file and later rereading a potentially changed copy.",
          "These checks depend on how the expected hashes were obtained and on the application binary that contains them. Matching hashes show that the bytes match the chosen model, not that the publisher or model is harmless. Producing normalized vectors with 384 dimensions shows that the model runs, not that it is safe. The trust document explains which evidence is cryptographic and which depends on procedure, environment, or publisher reputation.",
          "If an installed file fails verification, model loading stops without deleting the files or downloading replacements. The main window can open with semantic features unavailable. This does not change whether the app may write to the library. The model and database must both initialize before semantic features become available."
        ],
        "example": "An installed tokenizer file changes. The next load refuses before constructing the embedder, preserves the files for diagnosis, and does not reinterpret the change as a routine remote update.",
        "questions": [
          {
            "prompt": "A model file’s hash matches the manifest. What does that establish?",
            "answers": [
              {
                "text": "Its bytes match the expected file, assuming the manifest and application binary are trusted.",
                "feedback": "The comparison checks identity with the chosen artifact, not whether it is harmless.",
                "correct": true
              },
              {
                "text": "The publisher is proven harmless.",
                "feedback": "A hash comparison cannot establish intent or safety.",
                "correct": false
              },
              {
                "text": "Every later model version is approved automatically.",
                "feedback": "A new artifact needs its own expected hashes and review.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "An installed model file fails verification. How should the app respond?",
            "answers": [
              {
                "text": "Stop loading the model, preserve the files, and open with semantic features unavailable.",
                "feedback": "This check refuses the changed files; it does not automatically repair them or change library mode.",
                "correct": true
              },
              {
                "text": "Delete the files and silently download replacements.",
                "feedback": "The model-load verification path does neither.",
                "correct": false
              },
              {
                "text": "Accept the file if it still produces a vector.",
                "feedback": "Producing output does not replace the integrity check.",
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
        "title": "Review writes and queries at the intended library size",
        "objective": "Review large operations for bounded batches, safe retries, and evidence at the intended library size.",
        "paragraphs": [
          "Xaga’s design target is a library of millions of Entries, around four times expected intake. Entry is the code’s article type. A loop that works for one live edit can cause excessive database fragments or memory use during bulk import or recovery. Review the operation with large numbers of articles, not only a single example.",
          "Processing batches and saving checkpoints limit the work the app retains and must repeat after interruption. They do not automatically limit database-engine allocations or total process memory. LabelList and bitmap indexes help select matching records, but returning a large result still costs time and memory. Distinguish measurements from estimates and list excluded costs.",
          "For a change that writes data, identify its source files, locks, writer lanes, queue priority, ordering of writes, and recovery checks. Verify the affected security controls and keep exported APIs no wider than needed. If an assumption remains untested or duplicate logic remains separate, record the fact and its location. Do not silently decide its importance or when it must be fixed."
        ],
        "example": "For an operation changing many articles, specify the batch size, what is saved before each write, how retry works after each interruption, and how older queued work avoids overwriting newer data. Test those interruptions rather than checking only successful completion.",
        "questions": [
          {
            "prompt": "An operation retains at most 1,024 candidate article IDs at once. What can you conclude about memory?",
            "answers": [
              {
                "text": "That list is bounded; the rest of the process needs separate measurement.",
                "feedback": "Parsing, the database engine, caches, and other data can allocate additional memory.",
                "correct": true
              },
              {
                "text": "The whole process has a proven fixed memory ceiling.",
                "feedback": "A limit on one list is not a limit on every allocation.",
                "correct": false
              },
              {
                "text": "No interruption tests are needed.",
                "feedback": "Batch size does not establish correct saving and retry order.",
                "correct": false
              }
            ]
          },
          {
            "prompt": "How can you verify recovery for an operation that commits several writes?",
            "answers": [
              {
                "text": "Interrupt it between writes, retry it, and compare saved files with the repaired database.",
                "feedback": "That exercises the partial states the implementation must handle.",
                "correct": true
              },
              {
                "text": "Only check the success notification after an uninterrupted run.",
                "feedback": "That does not test interruption or retry.",
                "correct": false
              },
              {
                "text": "Time one article and assume the same rate for two million.",
                "feedback": "That neither tests recovery nor establishes performance at the target size.",
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
