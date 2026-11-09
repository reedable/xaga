// Curriculum content. Paths are relative to training/index.html.
const TRAINING = {
  product: {
    title: "Product owner / UX designer",
    description:
      "Understand Xaga’s purpose, domain, workflows, and product trade-offs.",
    lessons: [
      {
        id: "purpose",
        level: "Introduction",
        title: "From discovery to insight",
        objective: "Explain what Xaga is designed to help its user achieve.",
        paragraphs: [
          "Xaga is a personal knowledge management tool. It helps you save material worth reading, connect it to what you already know, and develop your own insights.",
          "When deciding what to build, ask how it will help the user learn. Receiving more articles or opening them faster can be useful, but neither tells you whether the user has gained a better understanding of the subject.",
          "The user does the thinking. Xaga helps them work with material, but they decide which claims to accept and what conclusions to draw.",
        ],
        example:
          "You propose a badge for “most articles opened today.” Before choosing that measure of success, ask whether opening more articles helps the user understand or connect them.",
        questions: [
          {
            prompt: "Which outcome best matches Xaga’s stated purpose?",
            answers: [
              {
                text: "A user connects an article to prior knowledge and develops an insight.",
                feedback:
                  "Reading, connecting ideas, and developing understanding are the outcomes described in Xaga’s README.",
                correct: true,
              },
              {
                text: "The user receives every article before anyone else.",
                feedback:
                  "Being first to receive an article does not establish that the user understands it.",
                correct: false,
              },
              {
                text: "Xaga guarantees that every saved article is true.",
                feedback:
                  "Xaga protects the handling of content; it cannot guarantee the truth of every article.",
                correct: false,
              },
            ],
          },
          {
            prompt: "Does saving an article in Xaga mean its claims are true?",
            answers: [
              {
                text: "No. The user still needs to judge the article’s claims.",
                feedback:
                  "Xaga helps you work with material. Saving an article does not verify its claims or decide what you should believe.",
                correct: true,
              },
              {
                text: "Yes. Saving the article means its arguments have been fact-checked.",
                feedback:
                  "Saving an article does not fact-check it. Judging its arguments remains the user’s responsibility.",
                correct: false,
              },
              {
                text: "Yes, if the article is popular.",
                feedback:
                  "Popularity does not establish whether a claim is true. The user still needs to assess it.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Product purpose",
            path: "../../README.md",
          },
          {
            title: "Secure by default",
            path: "../principles/secure_by_default.md",
          },
        ],
      },
      {
        id: "collections",
        level: "Introduction",
        title: "Browse by feed, Topic, or website",
        objective:
          "Distinguish the source of an article, a Topic, and browsing by website.",
        paragraphs: [
          "A Feed brings articles into your library. A Topic lets you group material that belongs together. You add Feeds and create Topics; each has its own saved record.",
          "A website can publish several feeds. Suppose example.com publishes one feed called “Arts” and another called “Entertainment”. A user can subscribe to either feed or both. Each Feed shows its own articles. Choosing example.com in Xaga’s Sites section brings together the saved articles from that website, regardless of which feed supplied them.",
          "These are different ways to browse the same library. Use a Feed to follow a particular subscription, a Topic to group related material, or the Sites section to browse by website. Xaga groups articles using their web addresses. This shows articles already in the library; it does not fetch every article on the website or subscribe the user to its other feeds.",
        ],
        example:
          "You subscribe to two feeds from example.com: “Arts” and “Entertainment”. Open “Arts” to browse articles from that feed, or “Entertainment” to browse articles from the other. Choose example.com in Sites to see the saved articles from both together.",
        questions: [
          {
            prompt:
              "You subscribe to two feeds from example.com: one called “Arts” and another called “Entertainment”. When you choose example.com in Xaga’s Sites section, which articles should appear?",
            answers: [
              {
                text: "Saved articles from example.com, regardless of which feed supplied them.",
                feedback:
                  "The Sites section groups articles already in the library by website, so it can bring together articles from several feeds.",
                correct: true,
              },
              {
                text: "Only articles from whichever feed you added first.",
                feedback:
                  "The feed’s subscription order does not affect this list. Saved articles from either feed can appear when their web addresses belong to example.com.",
                correct: false,
              },
              {
                text: "No articles until you manually add them to a group for example.com.",
                feedback:
                  "Xaga groups saved articles by their web addresses automatically. You do not add them to this list manually.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "You save your first article from example.com. Must you create a group for example.com before it appears in Xaga’s Sites section?",
            answers: [
              {
                text: "No. Xaga lists example.com automatically using the article’s web address.",
                feedback:
                  "The Sites section is populated from saved articles. Unlike a Topic, a group of articles from one website does not need to be created separately.",
                correct: true,
              },
              {
                text: "Yes. You must create and name the group yourself.",
                feedback:
                  "Xaga gets the website name from the article’s address and groups its saved articles automatically.",
                correct: false,
              },
              {
                text: "Yes. You must subscribe to a feed from that website first.",
                feedback:
                  "Browsing saved articles by website does not require subscribing to a feed from that website.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Organization model",
            path: "../architecture/organization_model.md",
          },
          {
            title: "How articles are grouped by website",
            path: "../architecture/sites.md",
          },
          {
            title: "Domain types",
            path: "../../src/domain.ts",
          },
        ],
      },
      {
        id: "markings",
        level: "Introduction",
        title: "Read status, bookmarks, and tags",
        objective: "Distinguish read status, bookmark color, and tags.",
        paragraphs: [
          "Read status, bookmarks, and tags answer different questions about an article. Read or unread records read status. A bookmark marks an article with a color. Tags are labels you choose to organize it.",
          "Bookmark colors are Red, Orange, Yellow, Green, Blue, and Violet. Changing the color keeps the article bookmarked; removing the color removes the bookmark. Xaga uses the color itself to record the bookmark rather than saving an extra yes/no value.",
          "Articles and annotations can each have several tags. Open a highlighted passage’s Annotation details and select Tags to assign labels directly to that annotation. Its parent and siblings keep their own assignments. Removing Research from one target leaves the Research tag available elsewhere. Saved includes bookmarked articles and annotations—highlighted passages with optional comments—so Saved is broader than a list of bookmarked articles.",
        ],
        example:
          "An unread article can be bookmarked Violet and have a tag named Research at the same time. In Tags browsing, each article or annotation must directly have every selected tag; assignments split across targets do not combine. Marking it as read should remove neither the bookmark nor the tag.",
        questions: [
          {
            prompt:
              "You change an article’s bookmark color from Blue to Violet. What happens?",
            answers: [
              {
                text: "The article stays bookmarked, now in Violet.",
                feedback:
                  "Giving the article another bookmark color keeps it bookmarked.",
                correct: true,
              },
              {
                text: "The article becomes unbookmarked until you turn bookmarking on again.",
                feedback:
                  "The color itself records the bookmark; there is no separate yes/no value to switch on.",
                correct: false,
              },
              {
                text: "The article becomes unread again.",
                feedback: "Bookmark color and read status are independent.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "An article has a tag named “Research”. You remove that tag from this article. What should happen?",
            answers: [
              {
                text: "This article loses the Research tag. The tag remains available elsewhere.",
                feedback:
                  "Removing a tag from one article does not delete the tag or the article. Other articles keep their tags.",
                correct: true,
              },
              {
                text: "Every article loses the Research tag.",
                feedback:
                  "You removed the tag from only this article. Deleting the tag everywhere is a separate action that requires confirmation.",
                correct: false,
              },
              {
                text: "This article loses both the Research tag and its bookmark.",
                feedback:
                  "Removing a tag does not change whether the article is bookmarked.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Read status, bookmarks, and color",
            path: "../architecture/organization_model.md#reading-state-bookmarks-and-color",
          },
          {
            title: "Tags",
            path: "../architecture/organization_model.md#tags",
          },
          {
            title: "Bookmark commands",
            path: "../../src-tauri/src/commands/bookmarks.rs",
          },
        ],
      },
      {
        id: "accessibility",
        level: "Introduction",
        title: "Accessibility is a product requirement",
        objective:
          "Include keyboard access, screen-reader feedback, and readable presentation in feature acceptance.",
        paragraphs: [
          "Xaga treats accessibility as a requirement of the product. A feature needs to work for people who navigate with a keyboard, use a screen reader, or need readable contrast. These are part of deciding whether the feature is complete.",
          "The Inspector provides a concrete example. Its drawer needs an accessible name, keyboard access to its controls, Escape to close, and focus kept inside while open. Closing returns focus to the control that opened it, with the reading pane as a fallback. This lets someone use the feature without a mouse and continue where they left off.",
          "A control’s status must be understandable without relying on one visual cue. Xaga’s tag buttons expose whether they are selected through `aria-pressed`, which screen readers can announce. Reload provides status text even when its animation is disabled. Include these behaviors in the feature’s design and acceptance criteria, then verify them rather than assuming the component library supplies them.",
        ],
        example:
          "A tag button changes color when selected, but does not expose that selection to a screen reader. The visible toggle works, yet the feature still needs its accessible selected state implemented and checked.",
        questions: [
          {
            prompt:
              "New tag controls work with a mouse, but their buttons cannot be reached with the keyboard. Is the feature complete?",
            answers: [
              {
                text: "No. Keyboard access is part of the feature’s requirements.",
                feedback:
                  "Xaga treats accessibility as required behavior, not an optional later enhancement.",
                correct: true,
              },
              {
                text: "Yes. Keyboard support can always be left for a later improvement.",
                feedback:
                  "Deferring keyboard support leaves a required way of using the feature unavailable.",
                correct: false,
              },
              {
                text: "Yes, if the buttons have readable labels.",
                feedback:
                  "Readable labels help, but do not make unreachable controls usable from the keyboard.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "A selected tag button changes color. What else should communicate its selected state to a screen reader?",
            answers: [
              {
                text: "Expose the state through `aria-pressed`.",
                feedback:
                  "Xaga’s tag buttons use aria-pressed so selection is available to assistive technology as well as visible on screen.",
                correct: true,
              },
              {
                text: "A stronger color change alone.",
                feedback:
                  "Color alone does not expose the selected state to a screen reader.",
                correct: false,
              },
              {
                text: "An animation when the tag is selected.",
                feedback:
                  "Motion does not replace accessible state and must not be the only cue.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Product accessibility commitment",
            path: "../../README.md#accessibility",
          },
          {
            title: "Inspector keyboard and focus behavior",
            path: "../architecture/article_inspector.md#responsive-presentation-and-keyboard-access",
          },
          {
            title: "Tag button selected states",
            path: "../architecture/tags.md#entry-assignment-api",
          },
          {
            title: "Motion and static feedback",
            path: "../conventions/motion.md",
          },
        ],
      },
      {
        id: "context",
        level: "Applied",
        title: "Keep your place while reading",
        objective:
          "Recognize when the list changes and when only an article’s details change.",
        paragraphs: [
          "When you are looking at unread articles and open one, Xaga marks it as read. It changes how the article appears but leaves it in the list. You can see what you just opened and continue without losing your place.",
          "The same rule applies to bookmark changes. Removing a bookmark from an article in an open Saved list does not immediately remove the article from that list. Choosing another list or changing a filter asks for a fresh set of matching articles.",
          "Removing a shortcut also leaves its current list and open article in place. A shortcut is a way to reach content, not the content itself. Deleting an article or collection is a different operation; this rule does not specify deletion behavior.",
        ],
        example:
          "Look at your unread articles and open the first one. It stays in the list after being marked read. If you later reopen the unread list, Xaga can show a new list that excludes articles you have read.",
        questions: [
          {
            prompt:
              "You’re looking at your unread articles and open one to read it. Xaga marks it as read. Should it disappear from the list, or stay visible?",
            answers: [
              {
                text: "Keep the article in the list and show that it has been read.",
                feedback:
                  "The article stays visible until you ask for a fresh list, for example by changing the filter or opening another list.",
                correct: true,
              },
              {
                text: "Remove the article from the list immediately.",
                feedback:
                  "Removing it immediately would make you lose your place while reading.",
                correct: false,
              },
              {
                text: "Keep showing the article as unread until Xaga restarts.",
                feedback: "The article should show that it has been read.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "You open a Topic through a shortcut, then remove that shortcut. What should happen?",
            answers: [
              {
                text: "The shortcut disappears, but you can keep reading the Topic’s articles.",
                feedback:
                  "Removing a shortcut does not close what you opened through it.",
                correct: true,
              },
              {
                text: "The articles are removed from the Topic.",
                feedback:
                  "A shortcut does not decide which articles belong to a Topic.",
                correct: false,
              },
              {
                text: "The open article closes.",
                feedback:
                  "Removing the shortcut preserves the open list and article.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "ListView behavior",
            path: "../architecture/list_view.md",
          },
          {
            title: "Navigation state",
            path: "../../src/hooks/useRouter.tsx",
          },
        ],
      },
      {
        id: "shortcuts",
        level: "Applied",
        title: "Shortcuts point to saved material",
        objective:
          "Explain what a shortcut saves and what happens when its destination is unavailable.",
        paragraphs: [
          "A shortcut can open an article, Feed, Topic, or the list of saved articles from a website. It points to existing content rather than making a copy. The library stores these references in `pins.yaml`.",
          "If Xaga cannot read a shortcut’s destination, it keeps the shortcut and offers Retry and Remove shortcut. A failed read does not establish that the content was deleted. Keeping the shortcut lets it work again if the content becomes available.",
          "A shortcut to example.com can still open an empty article list after all saved articles from that website are removed. That is different from an unreadable Topic. In a read-only library, viewing is allowed but saved changes are refused: you can open shortcuts and retry unavailable ones, but cannot remove a shortcut.",
        ],
        example:
          "A Topic cannot be read temporarily. Its shortcut stays visible. Retry tries to find the same Topic again; Remove shortcut removes only that route to it.",
        questions: [
          {
            prompt:
              "A Topic shortcut is still saved, but Xaga cannot currently read the Topic. What should the interface offer?",
            answers: [
              {
                text: "Keep the shortcut, with Retry and Remove shortcut actions.",
                feedback:
                  "The Topic may become readable again. A failed read is not proof that it was deleted.",
                correct: true,
              },
              {
                text: "Automatically remove the shortcut.",
                feedback:
                  "That would discard something the user saved, even if the Topic is only temporarily unavailable.",
                correct: false,
              },
              {
                text: "Create a new Topic with the same title.",
                feedback:
                  "A new Topic would not restore the original Topic and its contents.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "You added a shortcut to the saved articles from example.com. No articles from that website remain in the library. What should the shortcut do?",
            answers: [
              {
                text: "Open an empty article list for example.com, with zero counts.",
                feedback:
                  "The shortcut identifies the website. An empty list is still valid even when no saved articles match it.",
                correct: true,
              },
              {
                text: "The shortcut becomes permanently broken.",
                feedback:
                  "The website’s article list can be empty without being broken. New saved articles from example.com can appear there later.",
                correct: false,
              },
              {
                text: "Delete a separate file containing the website’s article list.",
                feedback:
                  "Xaga calculates the list from saved article addresses. There is no separate membership file to delete.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Pin storage rules",
            path: "../architecture/pins.md",
          },
          {
            title: "Shortcuts to articles from a website",
            path: "../architecture/sites.md#shortcuts",
          },
          {
            title: "Resolving shortcut destinations",
            path: "../../src/hooks/useShortcutTargets.ts",
          },
        ],
      },
      {
        id: "tags",
        level: "Applied",
        title: "Renaming and merging tags",
        objective: "Explain what renaming, merging, and deleting a tag change.",
        paragraphs: [
          "A tag has a name and a permanent ID. Renaming it changes the name while keeping it the same tag. Xaga ignores spaces at the start or end and compares names in lowercase. Research and RESEARCH match; straße and strasse remain different because lowercase conversion does not turn ß into ss.",
          "If you try to rename Research to Notes and Notes already exists, Xaga asks separately whether to merge them. A merge combines their assignments under the name Notes. It loses the distinction between articles that previously had Research, Notes, or both, so the confirmation must explain that this cannot be undone.",
          "Removing a tag from one article, deleting a tag everywhere, and deleting an article are separate actions. Global tag deletion removes article and annotation assignments but keeps the articles and annotations. Renames, merges, and deletions save progress so they can resume after interruption. They use indexed article and annotation assignments to find parent files; a file missed by that index can later cause an old tag name to reappear as a separate tag.",
        ],
        example:
          "You try to rename Research to Notes, but Notes already exists. Xaga should ask whether to merge them and explain the consequences. It must not silently merge them as part of the rename.",
        questions: [
          {
            prompt: "Which two tag names does Xaga treat as the same name?",
            answers: [
              {
                text: "Research and RESEARCH.",
                feedback:
                  "Xaga ignores letter case when comparing names but keeps the display spelling.",
                correct: true,
              },
              {
                text: "straße and strasse.",
                feedback:
                  "Converting letters to lowercase does not change ß to ss.",
                correct: false,
              },
              {
                text: "Any two names that look alike.",
                feedback:
                  "The comparison uses specified text rules, not visual similarity.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "You rename Research to Notes, but a tag named Notes already exists. What should Xaga do?",
            answers: [
              {
                text: "Ask separately whether to merge the tags and explain that their original assignments cannot be restored.",
                feedback:
                  "Merging changes which tags articles have and cannot be silently treated as a rename.",
                correct: true,
              },
              {
                text: "Merge the tags immediately without asking.",
                feedback:
                  "The requested name does not authorize an irreversible merge.",
                correct: false,
              },
              {
                text: "Delete the articles with the Research tag.",
                feedback: "Tag operations do not delete articles.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Tag naming and transitions",
            path: "../architecture/organization_model.md#tags",
          },
          {
            title: "Tag storage",
            path: "../architecture/tags.md",
          },
          {
            title: "Registry implementation",
            path: "../../crates/xaga-core/src/tags.rs",
          },
        ],
      },
      {
        id: "capabilities",
        level: "Applied",
        title: "Read-only mode is different from unavailable search",
        objective:
          "Distinguish an inability to save changes from an inability to use semantic features.",
        paragraphs: [
          "A read-only library lets you view existing data but refuses saved changes. This can happen because another process holds the library’s write lock or because the stored library version cannot be used for writing by this version of Xaga. Opening the library does not automatically upgrade it.",
          "Semantic search finds material by meaning. It uses an embedding model, which turns content into numerical representations, together with an embedding database. Related and relevance ranking also need these resources. Both must be ready before these features can work.",
          "Write access and semantic features can succeed or fail independently. Semantic search can work in a read-only library. A model failure can disable semantic search while ordinary library changes remain possible. During embedding startup the main window waits; if startup fails, the window opens with the affected features unavailable.",
        ],
        example:
          "The embedding model fails verification, but Xaga still has permission to write to the library. Ordinary reading and editing remain available; semantic search and Related do not.",
        questions: [
          {
            prompt: "Can semantic search work while a library is read-only?",
            answers: [
              {
                text: "Yes, if the existing embedding database and model are ready.",
                feedback:
                  "Searching existing data does not require permission to change the library.",
                correct: true,
              },
              {
                text: "No. Every search needs permission to write to the library.",
                feedback:
                  "Read-only Xaga processes can initialize a model and database for queries.",
                correct: false,
              },
              {
                text: "Only if opening the library automatically upgrades it.",
                feedback:
                  "Opening a library does not automatically upgrade it.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "The model fails verification, but Xaga still has permission to change the library. What should remain possible?",
            answers: [
              {
                text: "Ordinary library changes, while semantic search and related features are unavailable.",
                feedback:
                  "Model failure does not remove permission to write to the library.",
                correct: true,
              },
              {
                text: "Nothing can be changed because model failure makes the library read-only.",
                feedback:
                  "Library mode depends on whether Xaga holds its writer, not whether the model is ready.",
                correct: false,
              },
              {
                text: "Nothing can open until model loading succeeds.",
                feedback:
                  "An initialization failure allows the main window to open with the affected features unavailable.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Embedding startup and runtime capability",
            path: "../architecture/security/embedding_model_trust.md#runtime-failure-behavior",
          },
          {
            title: "Library opening",
            path: "../architecture/library_version.md#runtime-opening",
          },
          {
            title: "Startup provider",
            path: "../../src/providers/EmbeddingStartupProvider.tsx",
          },
        ],
      },
      {
        id: "article-refresh",
        level: "Applied",
        title: "Refreshing an externally edited article",
        objective:
          "Explain what happens to external body edits, annotations, and tags when you refresh an article.",
        paragraphs: [
          "You can edit a saved article’s Markdown file in a text editor outside Xaga. If you then choose “Refresh” in the article toolbar, Xaga replaces the body with a new copy from the website. Your body edits are lost, including added text, changed links, and inserted images.",
          "Annotations and tags you add in Xaga are preserved. To retain external body edits too, keep a separate copy before refreshing; Xaga does not merge them into the new body.",
          "This is article refresh. Feed refresh checks subscriptions for articles.",
        ],
        example:
          "You edit an article’s body outside Xaga and add an annotation and tag in Xaga. After article refresh, the annotation and tag remain, but your body edits are gone.",
        questions: [
          {
            prompt:
              "You edit an article’s Markdown body outside Xaga and add an annotation and tag in Xaga. After a successful article “Refresh”, what remains?",
            answers: [
              {
                text: "The annotation and tag remain; the external body edits are lost.",
                feedback:
                  "Refresh replaces the body without merging external edits. Annotations and tags are preserved.",
                correct: true,
              },
              {
                text: "The annotation, tag, and external body edits all remain.",
                feedback:
                  "Refresh preserves annotations and tags, but replaces the body, losing external edits.",
                correct: false,
              },
              {
                text: "Only the new body remains; the annotation and tag are removed.",
                feedback:
                  "Replacing the body does not remove annotations or tags.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "You edited an article’s body outside Xaga. You want to refresh it from the website and keep your edits for later. What should you do?",
            answers: [
              {
                text: "Keep a separate copy of your edits, then use the article’s “Refresh”.",
                feedback:
                  "The separate copy retains your edits when refresh replaces the body.",
                correct: true,
              },
              {
                text: "Use the article’s “Refresh” and rely on Xaga to merge your edits.",
                feedback:
                  "Refresh replaces the body; it does not merge external edits.",
                correct: false,
              },
              {
                text: "Refresh the feeds to replace the body and retain your edits.",
                feedback:
                  "Feed refresh checks subscriptions. It does not replace this article’s body.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Manual edits and article refresh",
            path: "../architecture/security/handling_persisted_content.md#manual-edits-and-article-refresh",
          },
          {
            title: "Article refresh control",
            path: "../../src/components/content/Toolbar.tsx",
          },
        ],
      },
      {
        id: "privacy",
        level: "Advanced",
        title: "A speed improvement can change privacy",
        objective:
          "Assess how a loading or performance change could let websites track the user.",
        paragraphs: [
          "Xaga’s secure-default rule is to start with secure settings. If a website needs a less secure option to work, the user must control that choice and understand its consequences. A loading failure must not silently relax the settings.",
          "In Settings → Privacy, Send referring URLs for feed redirects and Allow compressed feed transfers are off by default. The first discloses the previous feed URL to redirect destinations; the second advertises gzip support in exchange for potentially smaller transfers. Both apply to the next feed add, refresh, or import, not article-loader headers. Keep website sessions retains cookies and site data for listed websites when a new loader window opens.",
          "Settings → Security exposes Allow unencrypted HTTP, Allow HTTPS connections on other ports, and Maximum feed redirects. Read each consequence before changing it: HTTP permits traffic that can be read or changed in transit, extra CONNECT ports broaden reachable services, and additional feed redirects follow more website-selected destinations. Proxy controls apply to new proxy requests or tunnels, while the redirect cap applies to the next feed operation. These controls do not establish complete WebKit isolation. General groups size limits and timeouts; Appearance’s Reading subsection chooses the Default article view.",
          "Faster loading can come at a privacy cost. A website can recognize later requests if Xaga keeps cookies, connections, or other values the website supplied. One example is an ETag: a server-provided cache value that Xaga’s HTTP client sends back to check whether content changed. A unique ETag can identify that client and help the website link the user’s visits.",
          "Xaga limits retained state by creating HTTP clients for fetching feeds for each command and limiting the lifetime of article-loading webviews, its embedded browser windows. Reusing these resources needs a privacy review. Security claims also need evidence: native tests show cases where WebKit reaches loopback addresses on the user’s machine without using the configured proxy. The proxy therefore does not establish complete network isolation.",
        ],
        example:
          "Keeping an article-loading browser window alive may save startup time. Before accepting that change, identify which website-provided data would survive and whether a website could use it to recognize later visits.",
        questions: [
          {
            prompt:
              "A proposal would keep ETag values between feed fetches to avoid downloading unchanged content. How should that change be reviewed?",
            answers: [
              {
                text: "As a performance and privacy trade-off requiring an explicit decision.",
                feedback:
                  "A website can give Xaga’s HTTP client a unique ETag and recognize it when the same value returns on a later request.",
                correct: true,
              },
              {
                text: "As a speed improvement with no effect on privacy.",
                feedback:
                  "Keeping website-provided data between requests can let a website recognize the user.",
                correct: false,
              },
              {
                text: "As proof that the privacy defaults are defective.",
                feedback:
                  "The loading cost must be weighed against privacy; it does not justify changing the default silently.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "Native tests show that some WebKit requests bypass the configured proxy. Which statement is accurate?",
            answers: [
              {
                text: "The proxy checks requests that reach it, but does not prove all WebKit traffic is contained.",
                feedback:
                  "A proxy cannot check a request that bypasses it. The observed bypasses limit the security claim.",
                correct: true,
              },
              {
                text: "The proxy’s checks prove that every WebKit request is restricted.",
                feedback:
                  "The native tests demonstrate exceptions to that claim: some requests reached the local machine directly.",
                correct: false,
              },
              {
                text: "Removing unsafe HTML proves that an article’s claims are trustworthy.",
                feedback:
                  "HTML sanitization removes unsafe markup. It does not fact-check content or prove network isolation.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Secure defaults",
            path: "../principles/secure_by_default.md",
          },
          {
            title: "Resource lifetime and privacy",
            path: "../architecture/security/performance_tradeoff.md",
          },
          {
            title: "Fetching policy and enforcement gaps",
            path: "../architecture/security/fetching_remote_content.md",
          },
        ],
      },
      {
        id: "scale",
        level: "Advanced",
        title: "Know what a performance result measures",
        objective:
          "Tell measured performance apart from estimates and identify what a measurement leaves out.",
        paragraphs: [
          "Xaga expects roughly 240–360 thousand articles per year and is designed for around four times that intake: a library containing millions of articles. A small demonstration can show whether an interaction works, but not how it performs with a library that large.",
          "A performance result is useful only when you know what was timed. A database query might finish quickly while transferring, sorting, and displaying its results takes much longer. The design-scale document separates measurements from estimates and identifies work left out of each measurement.",
          "List virtualization means drawing only the items needed on screen. It can reduce drawing work while Xaga still receives and holds every matching result in memory. When assessing a list feature, specify the library size, number of matches, and whether the timing covers the complete user action.",
        ],
        example:
          "A tag query is quick when it returns a few hundred matches. That does not tell you how the UI behaves when clearing all tag selections returns millions of articles.",
        questions: [
          {
            prompt:
              "Xaga draws only the list items currently needed on screen. What does this limit?",
            answers: [
              {
                text: "How many items are drawn, but not necessarily how many results are loaded into memory.",
                feedback:
                  "Xaga can still receive all matching articles from the database.",
                correct: true,
              },
              {
                text: "All memory use, regardless of library size.",
                feedback: "The full query result may still occupy memory.",
                correct: false,
              },
              {
                text: "The need for database indexes.",
                feedback:
                  "Finding articles and drawing the visible list are separate work.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "A document estimates performance for a million articles using a smaller test. How should that result be presented?",
            answers: [
              {
                text: "As an estimate, with its assumptions and the work it leaves out.",
                feedback:
                  "An estimate is not a measurement of the complete operation.",
                correct: true,
              },
              {
                text: "As a measured guarantee for users.",
                feedback: "The larger operation has not been measured.",
                correct: false,
              },
              {
                text: "As irrelevant because the demonstration library is small.",
                feedback:
                  "Xaga is designed for libraries containing millions of articles.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Design target and measurement limits",
            path: "../architecture/design_scale.md",
          },
          {
            title: "Tag query scaling",
            path: "../architecture/design_scale.md#tag-query-scaling",
          },
          {
            title: "List rendering",
            path: "../../src/components/list/EntityList.tsx",
          },
        ],
      },
      {
        id: "review",
        level: "Advanced",
        title: "Review a proposal against Xaga’s behavior",
        objective:
          "Review a proposal against Xaga’s existing behavior and explain its consequences.",
        paragraphs: [
          "A proposal promises a cleaner interface. It would remove articles from the list as soon as they are read and discard shortcuts with unavailable destinations. It would also merge tags without asking when their names match.",
          "Each change affects a behavior taught in this track. The first makes the user lose their place. The second can discard a shortcut to temporarily unreadable content. The third performs an irreversible merge without confirmation. Review those consequences alongside the proposed visual simplicity.",
          "Explain the alternatives and evidence to the person making the product decision. If a change is approved, update the lasting documentation with its behavior. Task stories describe work still to do and are deleted when finished, so they cannot be the only record of an approved rule.",
        ],
        example:
          "For an action affecting many articles, ask what changes, what remains visible, whether it works read-only, and what happens if it stops halfway through. A simpler-looking control can still have complicated consequences.",
        questions: [
          {
            prompt:
              "How should you review the proposal to remove read articles, discard unavailable shortcuts, and merge matching tag names?",
            answers: [
              {
                text: "Explain which existing behaviors change and their consequences before a decision is made.",
                feedback:
                  "The changes affect what is saved and what the user can keep reading, not just appearance.",
                correct: true,
              },
              {
                text: "Implement it because fewer visible items always means simpler UX.",
                feedback:
                  "Showing fewer things does not by itself justify changing these behaviors.",
                correct: false,
              },
              {
                text: "Leave data-loss consequences out of the review because they are implementation details.",
                feedback:
                  "Consequences such as irreversible merge are necessary to the decision.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "Where should the lasting behavior of an approved change be recorded?",
            answers: [
              {
                text: "In the lasting documentation, with a task story describing only work still to do.",
                feedback:
                  "The documentation convention keeps product and engineering rules separate from unfinished tasks.",
                correct: true,
              },
              {
                text: "Only in an active task forever.",
                feedback:
                  "Task stories are deleted when finished, so lasting documentation must not rely on them.",
                correct: false,
              },
              {
                text: "Only in a screenshot.",
                feedback:
                  "A screenshot does not explain which data changes, what happens on failure, or how the feature behaves.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Decision ownership",
            path: "../principles/decision_ownership.md",
          },
          {
            title: "Documentation conventions",
            path: "../conventions/documentation.md",
          },
          {
            title: "Organization model",
            path: "../architecture/organization_model.md",
          },
        ],
      },
    ],
  },
  interface: {
    title: "UX designer / UI developer",
    description:
      "Connect interface behavior to Platform, saved changes, and safe content display.",
    lessons: [
      {
        id: "port",
        level: "Introduction",
        title: "How the UI uses Platform",
        objective:
          "Explain how UI components request work from the Rust backend.",
        paragraphs: [
          "A React component may need to mark an article as read or learn that its bookmark changed. In Xaga, it requests operations and listens for events through the `Platform` interface. The interface declares what the UI can ask for.",
          "`TauriPlatform` implements those methods by communicating with the Rust backend and converting returned data into UI objects. `main.tsx` creates that implementation and supplies it to components through `PlatformContext`. Keep native communication there rather than adding direct native calls to individual components.",
          "A TypeScript interface does not implement its methods or supply data by itself. Running the Vite server still leaves Xaga dependent on `TauriPlatform` and native startup calls, including the call that identifies the current window. Running without Rust would require a mock implementation and replacements for those startup calls.",
        ],
        example:
          "To mark an article read, a component calls `Platform.markEntryRead`. `TauriPlatform` sends the request to Rust, where the writer saves the change. The UI interface itself does not write the file.",
        questions: [
          {
            prompt:
              "A React component needs to mark an article as read. How should it request that operation?",
            answers: [
              {
                text: "Call Xagaropriate `Platform` method.",
                feedback:
                  "`TauriPlatform` handles the native communication behind that interface.",
                correct: true,
              },
              {
                text: "Add a direct native invoke call inside the component.",
                feedback:
                  "That would spread native communication outside the platform implementation.",
                correct: false,
              },
              {
                text: "Write to the library’s Markdown file from the browser.",
                feedback: "Library writes go through the Rust writer.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "Does starting the Vite development server alone let Xaga run without Rust?",
            answers: [
              {
                text: "No. Native startup calls and `TauriPlatform` still need replacements.",
                feedback:
                  "`main.tsx` creates `TauriPlatform` and reads the native window label.",
                correct: true,
              },
              {
                text: "Yes. A TypeScript interface automatically supplies sample data.",
                feedback:
                  "An interface declares methods; it does not implement them.",
                correct: false,
              },
              {
                text: "Yes. The training page already replaces Xaga’s backend.",
                feedback:
                  "The training page is separate from the React interface.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Platform interface",
            path: "../../src/platform/Platform.ts",
          },
          {
            title: "Tauri adapter",
            path: "../../src/platform/TauriPlatform.ts",
          },
          {
            title: "Application startup",
            path: "../../src/main.tsx",
          },
          {
            title: "Least visibility",
            path: "../principles/least_visibility_principle.md",
          },
        ],
      },
      {
        id: "identity",
        level: "Introduction",
        title: "Article summaries, loaded articles, and collections",
        objective:
          "Recognize Xaga’s UI types and use their fields according to their meaning.",
        paragraphs: [
          'Xaga separates an article summary from its loaded contents. `Entry` includes information such as its ID, title, URL, and read status. `Article` adds the body and annotations. Loading the body does not change the object’s role: both use `__kind: "entry"`.',
          "`Annotation` describes a highlighted passage and its optional comment. Its `content` field stores that comment. This is different from the separate `Note` type. An action on an annotation must identify that annotation, not just its containing article.",
          "Collections share browsing information without sharing every storage detail. `Entity` provides `id` and `title`; `ManagedEntity` adds fields such as `createdAt` for independently managed records. Feed and Topic have their own records and member lists. The `Site` data type describes articles grouped by a normalized website hostname, without its own creation date or saved member list. The Sites navigation section displays those groups; the `SiteList` component renders that sidebar section.",
          "A type guard checks the object’s fields together with its `__kind` value where required. The kind alone is not enough. A loaded member list can also be incomplete: use `entryCount` for the supplied total, rather than counting `members.length`.",
        ],
        example:
          "A Feed object has ten articles loaded in `members`, while `entryCount` reports 300. Show 300 as its article count. The ten loaded references do not establish the total number of articles.",
        questions: [
          {
            prompt:
              "A Feed’s loaded `members` list has ten articles, but `entryCount` reports more. Which value should its article count show?",
            answers: [
              {
                text: "`entryCount`.",
                feedback:
                  "The loaded `members` list can be incomplete; the supplied summary count covers the collection.",
                correct: true,
              },
              {
                text: "`members.length`.",
                feedback:
                  "That would count only the members loaded into this object.",
                correct: false,
              },
              {
                text: "The average of `entryCount` and `members.length`.",
                feedback:
                  "These describe different things. Averaging them does not produce a valid count.",
                correct: false,
              },
            ],
          },
          {
            prompt: "What does an `Article` object add to an `Entry` summary?",
            answers: [
              {
                text: 'The article body and annotations, while keeping `__kind: "entry"`.',
                feedback:
                  'An `Article` is an `Entry` with its content loaded. Its `__kind` stays `"entry"`.',
                correct: true,
              },
              {
                text: 'A new article kind that replaces `__kind: "entry"`.',
                feedback:
                  'Loading the body does not change the kind. `Article` still uses `__kind: "entry"`.',
                correct: false,
              },
              {
                text: "A `title` field, which an `Entry` summary never has.",
                feedback:
                  "`Entry` summaries already have a `title`. The added fields are content and annotations.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Domain types and type-checking functions",
            path: "../../src/domain.ts",
          },
          {
            title: "Type identity",
            path: "../architecture/type_identity.md",
          },
          {
            title: "Collections",
            path: "../architecture/organization_model.md#collections-and-identity",
          },
          {
            title: "Sites sidebar component",
            path: "../../src/components/sidebar/SiteList.tsx",
          },
        ],
      },
      {
        id: "motion",
        level: "Introduction",
        title: "Make the interface work without animation",
        objective: "Apply Xaga’s motion convention to controls and overlays.",
        paragraphs: [
          "Start with an interface that works without animation. In Xaga, base styles must show a complete, usable state. Add animation only inside `prefers-reduced-motion: no-preference`, the CSS query that matches when the user has not requested reduced motion.",
          "This order matters. If a browser does not support the query, it keeps the static base styles. If you animate by default and disable animation only for reduced motion, an unsupported query can leave animation running.",
          "The reload control shows how to apply the rule: its base styles disable animation, the query adds spinning, and its title and screen-reader status still say “Reload in progress...”. Apply the same approach to drawer movement, tab transitions, and animations supplied by component libraries.",
        ],
        example:
          "Turn off motion and activate Reload. The user should still know that work is in progress. The static status text must provide that information without relying on a spinning icon.",
        questions: [
          {
            prompt:
              "You add a busy animation to Xaga’s Reload control. Which CSS approach follows the motion convention?",
            answers: [
              {
                text: "Start without animation and enable it only inside `prefers-reduced-motion: no-preference`.",
                feedback:
                  "The interface then works without motion for both reduced-motion users and browsers that do not support the query.",
                correct: true,
              },
              {
                text: "Animate by default and disable it only inside `prefers-reduced-motion: reduce`.",
                feedback:
                  "If the browser does not support that query, the default animation remains enabled. Xaga requires a static base.",
                correct: false,
              },
              {
                text: "Use JavaScript to force animation regardless of the motion preference.",
                feedback:
                  "That would ignore the user’s motion preference instead of honoring it.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "The user has requested reduced motion, so Reload does not spin. How should they know it is still running?",
            answers: [
              {
                text: "Status text should explain that reload is in progress, including feedback for screen readers.",
                feedback: "Motion cannot be the only communication channel.",
                correct: true,
              },
              {
                text: "Nothing should indicate progress until the operation finishes.",
                feedback: "That leaves the static interface incomplete.",
                correct: false,
              },
              {
                text: "The control should be disabled without any explanation.",
                feedback:
                  "Disabling alone does not explain the operation’s state.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Motion convention",
            path: "../conventions/motion.md",
          },
          {
            title: "Reload icon implementation",
            path: "../../src/components/content/Toolbar.module.css",
          },
          {
            title: "Inspector motion and focus",
            path: "../architecture/article_inspector.md#responsive-presentation-and-keyboard-access",
          },
        ],
      },
      {
        id: "routing",
        level: "Applied",
        title: "Keep list navigation separate from article selection",
        objective:
          "Keep the current list and the selected article separate when handling navigation.",
        paragraphs: [
          "Opening an article and choosing a different list are separate actions. A list can continue showing the articles you were browsing while the reading pane displays one selected article or annotation. Marking an article read or changing its bookmark updates it without rebuilding that list.",
          "The router stores these choices separately. For an article shortcut, `articleScopeId` identifies the article whose list is open. `entryId` and `annotationId` identify the article or annotation currently selected in the reading pane. Selecting an annotation should move the reading pane to that passage without changing which article the list belongs to.",
          "Queries must follow the latest navigation. If a previous tag query finishes after the user changes their selection, ignore its result. Back and Forward restore selected tag IDs through navigation history; those IDs are not saved as library preferences.",
        ],
        example:
          "Open a pinned article and select an annotation in its list. The reading pane moves to that passage, while the list still contains the same article and its annotations.",
        questions: [
          {
            prompt:
              "Which action should leave the same articles in the list you are browsing?",
            answers: [
              {
                text: "Changing the bookmark color of an article already in that list.",
                feedback:
                  "Its bookmark changes, but the current list is preserved.",
                correct: true,
              },
              {
                text: "Choosing another Feed or Topic.",
                feedback: "Choosing another collection asks for a new list.",
                correct: false,
              },
              {
                text: "Changing which tags are selected.",
                feedback:
                  "Changing selected tags asks for a new matching set of articles.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "You change the selected tags while a query is running. The old query finishes after the new one. Which result should remain on screen?",
            answers: [
              {
                text: "Ignore the old result because it belongs to the previous selection.",
                feedback:
                  "The displayed list must match the user’s latest selection.",
                correct: true,
              },
              {
                text: "Display the old result because it arrived last.",
                feedback:
                  "Response order does not determine which selection is current.",
                correct: false,
              },
              {
                text: "Save both selections in `library.yaml`.",
                feedback:
                  "These selections belong to navigation history, not library-version metadata.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "List and article-shortcut behavior",
            path: "../architecture/list_view.md",
          },
          {
            title: "Router",
            path: "../../src/hooks/useRouter.tsx",
          },
          {
            title: "List implementation",
            path: "../../src/components/list/EntityList.tsx",
          },
        ],
      },
      {
        id: "inspector",
        level: "Applied",
        title: "Know what the Inspector is inspecting",
        objective:
          "Distinguish what is being inspected from where the Inspector appears.",
        paragraphs: [
          "The Inspector shows information about the selected article or annotation. Related and Topics are available for both; Tags is available for articles. Choosing an annotation in the reading pane does not open a closed Inspector. Opening Inspect is a separate action.",
          "For an article, the Inspector uses a 280-pixel side panel when the reading pane’s container is at least 700 CSS pixels wide. With less space it opens as a drawer over the window. Measure the reading pane’s container, not the whole browser window. An Annotation Inspector always uses the drawer, even on a wide layout.",
          "Opening and closing inspection must preserve the reading position. The drawer needs an accessible name, a Close action, Escape dismissal, and focus kept inside while it is open. On closing, return focus to its opener or the reading pane if the opener is gone. Article and Annotation tab choices remain separate for the session.",
        ],
        example:
          "A browser window is 1,000 pixels wide, but surrounding panes leave only 600 pixels for the reading pane. Article inspection uses the drawer because the reading pane itself is narrower than 700 pixels.",
        questions: [
          {
            prompt:
              "You inspect an Annotation while the reading pane is wide enough for an Article side panel. Where should Annotation inspection open?",
            answers: [
              {
                text: "In the Inspector drawer.",
                feedback: "Annotation inspection always uses the drawer.",
                correct: true,
              },
              {
                text: "In the Article side panel, replacing its Article.",
                feedback:
                  "The Article panel continues to inspect the Article; the drawer inspects the Annotation.",
                correct: false,
              },
              {
                text: "In a new browser window.",
                feedback: "That is not the documented interface.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "When should Article inspection use the side panel rather than a drawer?",
            answers: [
              {
                text: "When the reading pane’s container is at least 700 CSS pixels wide.",
                feedback:
                  "Measure the space available to the reading pane, after other panes take their space.",
                correct: true,
              },
              {
                text: "Whenever the browser window is at least 700 pixels wide.",
                feedback:
                  "The reading pane can be narrower than the browser window.",
                correct: false,
              },
              {
                text: "Whenever the article has a wide image.",
                feedback:
                  "Article image width does not determine the panel layout.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Inspector behavior",
            path: "../architecture/article_inspector.md",
          },
          {
            title: "Inspector implementation",
            path: "../../src/components/content/Inspector.tsx",
          },
          {
            title: "Selected article and Inspector layout",
            path: "../../src/components/content/EntityView.tsx",
          },
        ],
      },
      {
        id: "committed",
        level: "Applied",
        title: "Keep successful saves visible while the index catches up",
        objective:
          "Show successful Topic and tag changes even while the database is catching up.",
        paragraphs: [
          "The Inspector’s Topics view shows which Topics contain the selected article or annotation. After a successful addition, the saved file changes before the database necessarily catches up. Reading only the older database result could make the Topic appear unselected again, even though the save worked.",
          "`TopicsProvider` remembers successful additions and removals while a library is open. It applies those changes to results from the reverse index—the database lookup that finds which Topics contain an article or annotation. This remembered state survives closing and reopening the Topics view, but is discarded when the library session ends.",
          "Keep pending, successful, and failed work distinct. Before the first Topic list arrives, show Loading Topics…. If saving a membership change fails, keep the last saved selection and report the failure. If the user inspects a different article, results for the previous article must not overwrite the new display.",
        ],
        example:
          "Add an article to a Topic named Research and wait for the save to succeed. Close and reopen the Topics view before indexing finishes. Research should remain selected because the successful change is remembered.",
        questions: [
          {
            prompt:
              "You select a Topic to add an article, but the save fails. What should the Topics view show?",
            answers: [
              {
                text: "The last successfully saved selection, along with the error.",
                feedback:
                  "The UI should not show the change as saved when it failed.",
                correct: true,
              },
              {
                text: "The requested selection as if the save had succeeded.",
                feedback: "That would misrepresent what was saved.",
                correct: false,
              },
              {
                text: "No Topics at all.",
                feedback:
                  "One failed save does not mean the Topics are unavailable.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "The Topics list has not finished loading for the first time. What should the Topics view show?",
            answers: [
              {
                text: "Loading Topics….",
                feedback:
                  "A pending response does not mean there are no Topics.",
                correct: true,
              },
              {
                text: "Create a New Topic, implying no Topics exist.",
                feedback:
                  "That would confuse data that has not loaded with an empty list.",
                correct: false,
              },
              {
                text: "The list from the previously opened library.",
                feedback:
                  "Data from another library must not appear as this library’s Topics.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Showing saved Topic changes",
            path: "../architecture/article_inspector.md#topic-membership",
          },
          {
            title: "Topics provider",
            path: "../../src/hooks/useTopics.tsx",
          },
          {
            title: "Shared membership UI",
            path: "../../src/components/content/TopicMembership.tsx",
          },
          {
            title: "Tag assignment API",
            path: "../architecture/tags.md#entry-assignment-api",
          },
        ],
      },
      {
        id: "tags-view",
        level: "Applied",
        title: "Browse articles with several tags",
        objective:
          "Explain which articles appear when tags are selected and preserve those selections through updates.",
        paragraphs: [
          "Selecting Research and History in Tags shows articles carrying both tags. This is an intersection: every selected tag must be present. With no tags selected, Tags browsing shows articles under the current filters, rather than including annotations as the All items shortcut does.",
          "While tags are selected, Unread is hidden and a previous Unread filter is temporarily treated as All. Saved, All, and text filtering remain available. Clearing the selected tags restores Unread and its previous filter. The sidebar and narrow-layout header share the same selected tag IDs.",
          "A rename changes the displayed name while retaining the selected ID. If a selected tag is missing from refreshed tag data, keep it as “Unavailable tag” and allow deselection. Dropping it silently would show articles that no longer meet the original request. Explicitly deleting a tag removes its selection, but preserves the current list until the next refresh or navigation.",
        ],
        example:
          "Research and History are selected. Research temporarily disappears from refreshed tag data. Keep an Unavailable tag selection instead of silently changing the request to “History only.”",
        questions: [
          {
            prompt:
              "You select Research and History in Tags. Which articles should appear?",
            answers: [
              {
                text: "Articles with both tags.",
                feedback:
                  "Selecting several tags means every selected tag must be present.",
                correct: true,
              },
              {
                text: "Articles with either tag.",
                feedback:
                  "That would show articles missing one of the selected tags.",
                correct: false,
              },
              {
                text: "Annotations belonging to either Topic.",
                feedback:
                  "Tags browsing covers Entries, and Tags are different from Topics.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "A selected tag is missing from refreshed tag data. What should the UI do?",
            answers: [
              {
                text: "Keep it selected as Unavailable tag and allow the user to deselect it.",
                feedback:
                  "Silently dropping it would change which articles match the user’s selection.",
                correct: true,
              },
              {
                text: "Display its UUID in place of its name.",
                feedback:
                  "Internal IDs should not become user-facing tag names.",
                correct: false,
              },
              {
                text: "Clear all selected tags.",
                feedback:
                  "That would change the user’s selection without asking.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Tags browsing behavior",
            path: "../architecture/list_view.md#tag-browsing",
          },
          {
            title: "Article queries and tag storage",
            path: "../architecture/tags.md",
          },
          {
            title: "Router state",
            path: "../../src/hooks/useRouter.tsx",
          },
        ],
      },
      {
        id: "rendering",
        level: "Advanced",
        title: "Check saved content before displaying it",
        objective:
          "Explain why saved content is checked again before display and what those checks do.",
        paragraphs: [
          "A saved article can contain unsafe HTML, even if it was checked before saving. It may have been edited externally, imported, or written by an older version. Check it again when preparing it for display.",
          "`markdownPipeline` converts Markdown and embedded HTML to HTML, then uses `DOMPurify` to remove unsafe markup. That removal is called sanitization. `buildEntryHtml` prepares the document shown in the reading pane, and `HtmlReader` displays it in an iframe, an embedded document with restrictions on what it can do.",
          "Content Security Policy (CSP) supplies additional rules for scripts and resource loading. Each protection needs its own evidence. A test showing that an unsafe attribute is removed does not prove that WebKit blocks network requests. Native tests have found requests to loopback, the user’s own machine, that bypass the configured proxy.",
        ],
        example:
          "An externally edited article includes HTML with an event handler that could run code. The display pipeline must remove it, even though the file is already in the library.",
        questions: [
          {
            prompt:
              "Why remove unsafe markup again when displaying a saved article?",
            answers: [
              {
                text: "The saved file may have changed, and earlier checks do not guarantee it is safe to display now.",
                feedback:
                  "Read-time checks apply to all saved content, regardless of where it came from.",
                correct: true,
              },
              {
                text: "Only because it came from a remote feed.",
                feedback:
                  "Imported, locally edited, and user-authored saved content need the same checks.",
                correct: false,
              },
              {
                text: "To prove that the article’s claims are true.",
                feedback: "Removing unsafe markup does not fact-check content.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "A test confirms that unsafe HTML is removed before display. What does that result establish?",
            answers: [
              {
                text: "The tested HTML removal works; native network restrictions still need separate checks.",
                feedback:
                  "Native WebKit networking and other protections need separate tests.",
                correct: true,
              },
              {
                text: "WebKit can never make a request to the user’s own machine.",
                feedback: "The native docs record bypass observations.",
                correct: false,
              },
              {
                text: "The installed embedding model files have been verified.",
                feedback:
                  "Model files are verified separately in the backend; this HTML test says nothing about them.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Read-time content handling",
            path: "../architecture/security/handling_persisted_content.md",
          },
          {
            title: "Markdown sanitation",
            path: "../../src/text/markdownPipeline.ts",
          },
          {
            title: "HTML preparation for the reading pane",
            path: "../../src/text/buildEntryHtml.ts",
          },
          {
            title: "Reading-pane implementation",
            path: "../../src/components/content/HtmlReader.tsx",
          },
        ],
      },
      {
        id: "diagnostics",
        level: "Advanced",
        title: "Explain a loading failure without losing the article",
        objective:
          "Keep error messages and loading indicators associated with the correct article-loading attempt.",
        paragraphs: [
          "If an article reload fails, keep the saved article visible and explain the failure. Details opens a diagnostic record for that specific attempt. Its ID connects the UI and Rust records, and Copy diagnostics copies the record as plain-text JSON rather than executable HTML.",
          "Keep later notifications separate. A warning about a website icon joins the queue instead of replacing the current notification or its Details link. A notification with diagnostics stays until dismissed; opening or closing Details does not dismiss it.",
          "Loading state also belongs to a specific attempt. Navigating from Article A to Article B cancels A’s frontend attempt; a late result from A must not clear B’s loading indicator. Read diagnostic messages literally: “image capture enabled” reports a setting, not that a download started or succeeded. History is size-limited, so a missing event is not proof that work never happened.",
        ],
        example:
          "Article A fails to reload. While its error notification is visible, another article produces an website-icon warning. The first notification must still open A’s diagnostic record.",
        questions: [
          {
            prompt:
              "A warning about a website icon arrives while an error notification with Details is visible. What should happen?",
            answers: [
              {
                text: "Queue the warning without replacing the current notification or its Details link.",
                feedback:
                  "Each notification must remain connected to its own loading attempt.",
                correct: true,
              },
              {
                text: "Replace the current notification with the latest message.",
                feedback:
                  "That could remove access to the error the user is investigating.",
                correct: false,
              },
              {
                text: "Display the warning’s technical data as HTML.",
                feedback:
                  "Diagnostic data must be displayed and copied as plain text.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "A diagnostic record says “image capture enabled.” Does that mean an image was downloaded?",
            answers: [
              {
                text: "No. The setting allowed capture; a separate result is needed to show a download happened.",
                feedback:
                  "Separate diagnostic records report whether capture was allowed, attempted, and completed.",
                correct: true,
              },
              {
                text: "Yes. It means every article image was downloaded.",
                feedback: "Enabled is not a result.",
                correct: false,
              },
              {
                text: "Yes. It means no image download could have failed.",
                feedback: "The preference says nothing about outcomes.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Article-loading diagnostics",
            path: "../architecture/article_loading_diagnostics.md",
          },
          {
            title: "Frontend loading-attempt tracking",
            path: "../../src/utils/ArticleLoadAttempt.ts",
          },
          {
            title: "Native diagnostic collector",
            path: "../../src-tauri/src/article_diagnostics.rs",
          },
        ],
      },
      {
        id: "review",
        level: "Advanced",
        title: "Review the behavior a screenshot cannot show",
        objective:
          "Review behavior when requests overlap or fail, as well as when they succeed.",
        paragraphs: [
          "An interface review needs more than a successful screenshot. Check which article or annotation is selected, which list is open, what has actually been saved, which requests are unfinished, and where keyboard focus goes.",
          "Use repeatable examples for loading, empty, unavailable, read-only, and failed states. These are different situations. A list that has not loaded should not claim to be empty, and a failed save should not look successful.",
          "Test the part that changed. Component tests can check the Inspector selection and focus; navigation tests can check late query results; native WebKit tests can check engine-specific behavior. Keep helpers local until another file needs them, so a UI refactor does not expose unnecessary APIs.",
        ],
        example:
          "Begin inspecting Article A, then switch to Article B before A’s lookup finishes. The completed lookup for A must not replace B’s information. Repeat with an Annotation and check focus when the drawer closes.",
        questions: [
          {
            prompt:
              "Which test could reveal a wrong Inspector selection that a screenshot misses?",
            answers: [
              {
                text: "Switch to another inspected article or annotation before the first lookup finishes.",
                feedback:
                  "The earlier result must not overwrite the newly inspected object.",
                correct: true,
              },
              {
                text: "Display one fully loaded article and stop there.",
                feedback:
                  "That does not test what happens when requests overlap.",
                correct: false,
              },
              {
                text: "Replace every empty state with a spinner.",
                feedback:
                  "That hides the difference between loading and genuinely empty data.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "A helper function is used only within one file. Should it be exported?",
            answers: [
              {
                text: "Keep it local until another file actually needs it.",
                feedback:
                  "This avoids creating an unnecessary dependency other files can rely on.",
                correct: true,
              },
              {
                text: "Export it because another file might need it someday.",
                feedback:
                  "A hypothetical caller does not justify a larger public interface.",
                correct: false,
              },
              {
                text: "Add an `index.ts` file that re-exports every helper.",
                feedback:
                  "The project rejects broad re-export files that expose more than callers need.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Inspector behavior",
            path: "../architecture/article_inspector.md",
          },
          {
            title: "Least visibility",
            path: "../principles/least_visibility_principle.md",
          },
          {
            title: "Semantic vocabulary",
            path: "../conventions/vocabulary/semantic_vocabulary.md",
          },
          {
            title: "Inspector interaction tests",
            path: "../../src/components/content/EntityView.inspector.test.tsx",
          },
        ],
      },
    ],
  },
  engineering: {
    title: "Architect / Backend engineer",
    description:
      "Understand how the backend saves data, schedules work, recovers from interruption, and checks untrusted input.",
    lessons: [
      {
        id: "layers",
        level: "Introduction",
        title: "Follow a change from command to file to database",
        objective:
          "Identify which work belongs in core, a Tauri command handler, and the CLI.",
        paragraphs: [
          "When the UI asks Rust to do something, a Tauri command handler receives the request. In Xaga, the handler coordinates the operation: call core to save the change, wait for success, then queue any database update that follows.",
          "`xaga-core` implements file operations and shared logic. Those operations do not choose embedding jobs. The command-line interface (CLI) handles terminal input and output, database management, and explicit library upgrades. Put each part where its responsibility belongs, regardless of which caller needed it first.",
          "The bookmark commands show the sequence. A handler gets `LibraryWriter`, waits for the color to be saved, and queues `SyncState` with that saved color. The database receives the known change without inferring it from a file event. Likewise, shared validation can belong in core while CLI progress printing remains in the CLI.",
        ],
        example:
          "For a bookmark-color change, identify three steps: the command receives the request, the writer saves the file, and a queued job updates the database. A failed file write must not be reported as a successful change.",
        questions: [
          {
            prompt:
              "A bookmark command has successfully saved the new color to the article file. Where should `SyncState` be queued?",
            answers: [
              {
                text: "In the command handler, after the writer succeeds.",
                feedback:
                  "The handler coordinates saving the file and updating its database copy.",
                correct: true,
              },
              {
                text: "Inside core’s file-patching function.",
                feedback:
                  "Core’s file operations do not manage the embedding queue.",
                correct: false,
              },
              {
                text: "In a file watcher that infers what changed.",
                feedback:
                  "The command already knows the action and its saved result.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "A CLI command needs validation and progress printing. Which part belongs in core?",
            answers: [
              {
                text: "Validation, with printing left to the CLI.",
                feedback:
                  "Core can share the logic without taking responsibility for console output.",
                correct: true,
              },
              {
                text: "Console formatting because the CLI first needed the operation.",
                feedback:
                  "The first caller does not decide which layer owns each responsibility.",
                correct: false,
              },
              {
                text: "Argument parsing and exit-code decisions.",
                feedback: "Those belong to the command-line interface.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Layer ownership",
            path: "../architecture/layer_ownership.md",
          },
          {
            title: "Carry intent end-to-end",
            path: "../principles/carry_intent_end_to_end.md",
          },
          {
            title: "Bookmark orchestration",
            path: "../../src-tauri/src/commands/bookmarks.rs",
          },
        ],
      },
      {
        id: "authority",
        level: "Introduction",
        title: "Know which files own the data",
        objective:
          "Identify which file owns a value before changing it or rebuilding its database copy.",
        paragraphs: [
          "Xaga saves library content in files and uses LanceDB to query a database copy. The files are the source of truth: recovery must be able to rebuild the database from them. Command handlers apply saved user actions to the database; recovery reconciles it with the files.",
          "Different files own different facts. `Entry` is the article type used by the code. Its Markdown file stores article content, read status, color, tags, and annotations. Feed Atom files store Feed membership. Topic Markdown stores ordered member references. `pins.yaml` stores shortcuts, `tags.yaml` stores tag IDs and unfinished tag operations, and `library.yaml` stores library-version information.",
          "The direction of a lookup matters. A Topic file answers “which articles or annotations belong to this Topic?” The reverse database index answers “which Topics contain this article or annotation?” That reverse lookup cannot replace the ordered member list. A content checksum—a value used to detect changed content—also cannot detect every membership or state change.",
        ],
        example:
          "Add an article to a Topic without editing the article. The Topic file changes, but the article’s content checksum can stay the same. Recovery must still update the reverse Topic lookup.",
        questions: [
          {
            prompt:
              "Which saved data determines a Topic’s members and their order?",
            answers: [
              {
                text: "The member references in its Topic Markdown file.",
                feedback:
                  "The reverse database index supports lookup in the opposite direction.",
                correct: true,
              },
              {
                text: "Only the topics column on each member’s database row.",
                feedback:
                  "That column cannot replace the Topic’s ordered, typed member references.",
                correct: false,
              },
              {
                text: "Whatever the last UI component loaded.",
                feedback:
                  "The UI displays membership; it does not own the saved list.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "The article text has not changed, so its content checksum is unchanged. Why might recovery still need to update LanceDB?",
            answers: [
              {
                text: "Read status, color, or Topic membership can change without changing the article content.",
                feedback:
                  "Recovery needs checks that detect these changes separately.",
                correct: true,
              },
              {
                text: "Checksums are never useful.",
                feedback:
                  "A checksum can detect changes to the content it covers.",
                correct: false,
              },
              {
                text: "After startup, the database replaces the files as the source of truth.",
                feedback:
                  "The database remains a copy that can be rebuilt from files.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Single source of truth",
            path: "../principles/single_source_of_truth.md",
          },
          {
            title: "Recovery overview",
            path: "../../crates/xaga-core/src/recovery/README.md",
          },
          {
            title: "Topic commands",
            path: "../../src-tauri/src/commands/topics.rs",
          },
        ],
      },
      {
        id: "locks",
        level: "Introduction",
        title: "Protect a library and serialize edits to each file",
        objective:
          "Distinguish the library lock, per-file write queues, and the lock for shared settings.",
        paragraphs: [
          "The library lock prevents two cooperating processes from writing the same library. It is an exclusive operating-system `flock` on `xaga.lock`, tied to an open file handle. The OS releases it when that handle closes, including on process exit. A leftover lock file does not mean the lock is still held.",
          "One process can still have two operations trying to edit the same article. `LibraryWriter` prevents that conflict with a per-file lane: a queue that reads the current file, applies a change, and saves it before starting the next operation. Other files can be processed in parallel. Atomic rename prevents a half-written file from being visible; it does not by itself prevent one edit overwriting another.",
          "The account lock on `settings.json` protects settings and model downloads shared across libraries. It is separate from a particular library’s lock. Operations inside the process share it, and a settings replacement transfers the lock to the new file without a gap. A stale-view check also detects settings that changed since the caller read them.",
        ],
        example:
          "Two Annotation edits start for the same article. Even though the process holds the library lock, they must use the same file queue. Otherwise both could read the old article and the second save could erase the first edit.",
        questions: [
          {
            prompt:
              "Why does a process holding the library lock still need a separate write queue for each file?",
            answers: [
              {
                text: "Its own concurrent operations could otherwise read the same old file and overwrite one another’s edits.",
                feedback:
                  "The library lock excludes other processes. The per-file queue also prevents overlapping edits inside the one writing process.",
                correct: true,
              },
              {
                text: "Atomic rename lets readers see half-written files.",
                feedback:
                  "Atomic rename prevents readers from seeing partial file contents. It does not prevent two operations from saving competing edits.",
                correct: false,
              },
              {
                text: "Every file needs a separate process.",
                feedback:
                  "The same process can handle many files, with one serialized queue for each file.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "Two Xaga processes open different libraries but both update `settings.json`. What coordinates their writes?",
            answers: [
              {
                text: "The account lock on `settings.json`.",
                feedback:
                  "Settings are shared across libraries for the same machine and user.",
                correct: true,
              },
              {
                text: "Only the first library’s `xaga.lock`.",
                feedback:
                  "That lock protects its library, not shared settings.",
                correct: false,
              },
              {
                text: "Nothing, because the library paths differ.",
                feedback:
                  "The Xaga processes still share settings and model resources.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Library lock and writer lanes",
            path: "../architecture/single_writer_library_lock.md",
          },
          {
            title: "Account lock and rotation",
            path: "../architecture/single_writer_account_lock.md",
          },
          {
            title: "Writer implementation",
            path: "../../crates/xaga-core/src/library_writer/mod.rs",
          },
          {
            title: "Atomic files and locks",
            path: "../../crates/xaga-core/src/fs.rs",
          },
        ],
      },
      {
        id: "priority",
        level: "Applied",
        title: "Choose queue priority by urgency",
        objective:
          "Assign queue priority by urgency and preserve each queue’s shutdown rules.",
        paragraphs: [
          "A queued `Job` describes the work; its `Tier` determines which work runs first. The embedding service handles Control, Interactive, Regular, and Maintenance in that order. Choose the tier by who needs the result and how soon, not simply by how expensive the job is.",
          "When the user adds a feed, they are waiting for articles to appear. The database update can therefore be Interactive, while creation of embedding vectors runs as Regular. A vector is a numerical representation of content used for semantic features. Recovery can create vectors as Maintenance because that work is not serving a newly requested result.",
          "`LibraryWriter` has its own Interactive and Regular queues, with different shutdown rules from the embedding service. Its Interactive backlog must finish because it can contain unsaved user input. Regular backlog may be dropped, so it must contain only work that can be recreated. Queued database work in the embedding service can be dropped and repaired by recovery.",
        ],
        example:
          "A large operation requested by the user can be urgent; a small background repair can wait. `Shutdown` also needs immediate attention even when finishing it is expensive.",
        questions: [
          {
            prompt:
              "The same embedding job can run after a user save or during background recovery. What should determine its priority?",
            answers: [
              {
                text: "Who needs the result and how urgently they need it.",
                feedback: "The cost of the work is a separate consideration.",
                correct: true,
              },
              {
                text: "Whether the action runs the model.",
                feedback: "The same kind of action can have different urgency.",
                correct: false,
              },
              {
                text: "Always putting large batches in Maintenance.",
                feedback:
                  "A user can be waiting for a result from a large operation.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "Which work is safe to put in `LibraryWriter`’s Regular queue?",
            answers: [
              {
                text: "Work that can be recreated if its queued write is dropped during shutdown.",
                feedback:
                  "Regular backlog may be dropped; unrecoverable user input must not be placed there.",
                correct: true,
              },
              {
                text: "An unsaved Annotation because it is small.",
                feedback:
                  "A small amount of user-authored content can still be irreplaceable.",
                correct: false,
              },
              {
                text: "Any write, because recovery can recreate lost user input.",
                feedback:
                  "Recovery works from saved files; it cannot recreate input that never reached them.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Priority definition",
            path: "../principles/definition_of_priority.md",
          },
          {
            title: "Embedding queue and jobs",
            path: "../../crates/xaga-core/src/embeddings/service.rs",
          },
          {
            title: "Writer admission and shutdown",
            path: "../architecture/single_writer_library_lock.md#in-process-write-serialization",
          },
        ],
      },
      {
        id: "crashes",
        level: "Applied",
        title: "Plan for interruption between writes",
        objective:
          "Describe what survives interruption between writes and how recovery detects unfinished work.",
        paragraphs: [
          "If the process is forcibly terminated, it can stop between any two saved writes. Plan for the file to have changed while the database has not. Graceful shutdown is a separate case: the embedding service handles `Shutdown` between processing iterations, but that handling cannot protect against a crash inside an iteration.",
          "Write order decides what survives. Feed persistence saves article files before listing them in the feed’s membership file. Image bytes are saved before Markdown refers to them. An unused saved image can be cleaned up; a saved reference without its bytes is unusable.",
          "Recovery needs a check for each unfinished change. It compares read status and color separately from content checksums. A late Annotation embedding must also check the current database row before updating its vector. If the Annotation was deleted or changed, the old job must not recreate it or overwrite its newer data.",
        ],
        example:
          "Xaga saves a new bookmark color, then crashes before `SyncState` runs. On restart, recovery finds the saved color in Markdown and updates the database. Unchanged article text does not need another embedding.",
        questions: [
          {
            prompt:
              "Xaga is forcibly terminated after saving an article’s color but before updating LanceDB. What should happen on recovery?",
            answers: [
              {
                text: "Read the saved color and repair the database row.",
                feedback:
                  "The file survived, and recovery checks color separately from article content.",
                correct: true,
              },
              {
                text: "Replace the file’s color with the old database value.",
                feedback:
                  "That would make the derived database override the source file.",
                correct: false,
              },
              {
                text: "Assume no partial state is possible because both steps ran in one service-loop iteration.",
                feedback:
                  "Forced termination can interrupt the operation between writes.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "An Annotation is deleted while an older embedding job for it is still running. Which outcome would be a bug?",
            answers: [
              {
                text: "The old job recreates the deleted Annotation database row or writes older metadata back.",
                feedback:
                  "An old result must not undo a later deletion or restore older data.",
                correct: true,
              },
              {
                text: "The old job checks whether the database row still exists before updating it.",
                feedback:
                  "Checking the current database row is required to avoid recreating deleted data.",
                correct: false,
              },
              {
                text: "The job changes only the vector on a database row that still matches its input.",
                feedback:
                  "For an existing database row that still matches the input and needs its vector, this limited update is allowed.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Crash consistency",
            path: "../architecture/crash_consistency.md",
          },
          {
            title: "Queued Annotation completion",
            path: "../architecture/crash_consistency.md#queued-annotation-vector-completion",
          },
          {
            title: "Persistence implementation",
            path: "../../crates/xaga-core/src/persistence/articles.rs",
          },
        ],
      },
      {
        id: "transitions",
        level: "Applied",
        title: "Record unfinished work before changing many files",
        objective:
          "Explain how tag operations record unfinished work and what their completion does not prove.",
        paragraphs: [
          "A tag rename can affect many article files, so it cannot be treated as one indivisible save. Xaga records the tag’s ID, names, and unfinished operation in `tags.yaml`. Article files keep readable tag names; LanceDB keeps the corresponding IDs.",
          "Before changing a batch of files, save which article IDs are pending. After the batch succeeds, save a checkpoint describing completed progress. Retry can then finish a partly processed batch. Each article still uses its own writer lane, and the registry uses the tags lane. Revision checks prevent an older worker from advancing newer progress.",
          "The operation uses indexed candidates and saved pending work, rather than checking every source file for completion. A missed article can keep an old tag name and later cause discovery to create a separate tag with that name. Names are reserved while the operation is active to prevent conflicts; completion releases the reservations.",
        ],
        example:
          "Research is being deleted from a batch of articles. Save the pending article IDs before editing those files. If the process stops halfway through, recovery can retry those IDs even if the database has since been rebuilt.",
        questions: [
          {
            prompt:
              "A tag operation is about to change its next batch of article files. When should it save the IDs of those pending articles?",
            answers: [
              {
                text: "Before changing those files.",
                feedback:
                  "If the process stops during file changes, the batch must already be saved for retry.",
                correct: true,
              },
              {
                text: "Only after the entire library is processed.",
                feedback:
                  "That would leave no saved batch to resume after interruption.",
                correct: false,
              },
              {
                text: "Only in LanceDB.",
                feedback:
                  "The operation must remain resumable even when the database cache is rebuilt.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "A tag rename finishes, but an article missing from the index still has the old tag name. What can happen later?",
            answers: [
              {
                text: "Discovery can create a separate tag with that old name.",
                feedback:
                  "Completion does not scan every file or keep the old name reserved permanently.",
                correct: true,
              },
              {
                text: "The rename’s completion proves that every article file was checked.",
                feedback:
                  "Completion covers indexed candidates and pending work, not a complete source scan.",
                correct: false,
              },
              {
                text: "The old name remains permanently blocked.",
                feedback: "Completion releases the name reservation.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Tag storage and unfinished operations",
            path: "../architecture/tags.md",
          },
          {
            title: "Checkpoint ordering",
            path: "../architecture/crash_consistency.md#tag-definitions-and-transitions",
          },
          {
            title: "Rename implementation",
            path: "../../crates/xaga-core/src/tags/rename.rs",
          },
          {
            title: "Merge implementation",
            path: "../../crates/xaga-core/src/tags/merge.rs",
          },
        ],
      },
      {
        id: "versions",
        level: "Applied",
        title: "Version the library, not just Xaga",
        objective:
          "Explain when a library version changes and why opening a library does not upgrade it.",
        paragraphs: [
          "Xaga’s application version and a library’s stored format version are related but separate facts. Each library records its completed `major.minor` in `library.yaml`. A patch, prerelease, or build suffix does not define a new library format. Installing a new version of Xaga does not rewrite existing library versions.",
          "Before 1.0, the CLI explicitly upgrades a library while holding its write lock. It records the target version before conversion starts and keeps the last completed version until both files and database have been reconciled. If interrupted, retry must continue toward the same target.",
          "Xaga checks this metadata before starting a writer or embedding service. A missing, different, incomplete, or unreadable version opens read-only without rewriting the manifest, creating a cache, or running recovery. A missing manifest does not prove any particular older format. During major version 0, a storage-model change requires a minor-version increment.",
        ],
        example:
          "An upgrade from library format 0.13 to 0.14 stops halfway through. The manifest still records completed 0.13 and pending 0.14. A retry must finish conversion and reconciliation before marking 0.14 complete.",
        questions: [
          {
            prompt:
              "When may an upgrade mark the target library version as complete?",
            answers: [
              {
                text: "After the files are converted and the database has been updated to agree with them.",
                feedback:
                  "The target is complete only when both conversion and database reconciliation have succeeded.",
                correct: true,
              },
              {
                text: "As soon as the user selects the target version.",
                feedback:
                  "The target must remain pending while conversion or reconciliation is unfinished.",
                correct: false,
              },
              {
                text: "Whenever a newer version of Xaga opens the library.",
                feedback:
                  "Opening Xaga does not perform an explicit upgrade or change a library’s stored version.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "Xaga opens a library with no version manifest. What should startup do?",
            answers: [
              {
                text: "Open read-only without creating version metadata or guessing an older format.",
                feedback:
                  "An explicit upgrade is needed to establish the library version.",
                correct: true,
              },
              {
                text: "Assume the oldest supported format and save that version immediately.",
                feedback:
                  "A missing file does not establish which format the library uses.",
                correct: false,
              },
              {
                text: "Use the version of the library opened before this one.",
                feedback: "Each library has its own version metadata.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Library version rules",
            path: "../architecture/library_version.md",
          },
          {
            title: "Explicit upgrade",
            path: "../library_upgrade.md",
          },
          {
            title: "Release and storage versions",
            path: "../development/release_cycle.md",
          },
          {
            title: "Manifest API",
            path: "../../crates/xaga-core/src/library/manifest.rs",
          },
        ],
      },
      {
        id: "security",
        level: "Advanced",
        title: "Check network destinations and native command arguments",
        objective:
          "Identify what untrusted callers can make a network request or native command do.",
        paragraphs: [
          "A network request starts with a destination hostname. DNS can return several IP addresses for it. Xaga’s proxy checks every address and refuses the whole destination if any is disallowed. It connects to a validated address without resolving the hostname again. These checks apply to requests reaching the proxy; native WebKit tests have shown loopback requests that bypass it.",
          "Destination checks do not make returned content safe. Captured article bodies are sanitized before saving: unsafe markup is removed. Stored content is checked again before display. Resource limits and model verification address other risks. A passing check at one step is not evidence for every other step.",
          "Native commands also need to restrict what the caller can choose. `command_registry.json` defines Tauri command registration and permissions, while window and library-write checks add separate restrictions. Keep a user-selected import path in Rust from file dialog through file operation. Letting JavaScript supply an arbitrary path could let it read or overwrite a different file.",
        ],
        example:
          "The import menu opens a native file dialog in Rust, then passes the selected path directly to the Rust import function. A compromised webview cannot use that flow to supply its own arbitrary path.",
        questions: [
          {
            prompt:
              "DNS returns both public and private IP addresses for one hostname. Why does the proxy refuse the destination?",
            answers: [
              {
                text: "Every address must be allowed; a public address must not hide a disallowed one.",
                feedback:
                  "The proxy checks the whole result before connecting to a validated address.",
                correct: true,
              },
              {
                text: "Only the first address matters.",
                feedback:
                  "Another returned address could lead to a disallowed destination.",
                correct: false,
              },
              {
                text: "Refusing the address proves that all page HTML is safe.",
                feedback: "Destination checks do not check the page’s markup.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "Where should an import operation handle the path selected in a file dialog?",
            answers: [
              {
                text: "In Rust, keeping the dialog and the authorized file operation together.",
                feedback:
                  "The webview should not be able to supply an arbitrary filesystem path.",
                correct: true,
              },
              {
                text: "In a webview-callable command that reads any path JavaScript passes.",
                feedback:
                  "That would let the caller choose files outside the intended operation.",
                correct: false,
              },
              {
                text: "Only in CSS.",
                feedback:
                  "CSS cannot carry out or authorize native file access.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Fetching controls and gaps",
            path: "../architecture/security/fetching_remote_content.md",
          },
          {
            title: "Command surface",
            path: "../architecture/security/command_surface.md",
          },
          {
            title: "Command registry",
            path: "../../src-tauri/command_registry.json",
          },
          {
            title: "Native imports",
            path: "../../src-tauri/src/native/feeds.rs",
          },
          {
            title: "Checking saved content before display",
            path: "../architecture/security/handling_persisted_content.md",
          },
        ],
      },
      {
        id: "model",
        level: "Advanced",
        title: "Understand what model verification establishes",
        objective:
          "Explain what model checksums establish, what they do not, and how verification failure behaves.",
        paragraphs: [
          "Xaga’s model manifest records an exact revision and expected SHA-256 hashes. A hash is a value calculated from a file’s bytes. Xaga compares downloaded files with the expected hashes before installation, and checks installed files again before loading them into fastembed or ONNX Runtime.",
          "The loader uses the same bytes it checked, rather than checking one file and rereading a potentially changed copy. Matching hashes establish that these are the selected model files. They do not establish that the model or its publisher is harmless. The expected hashes and Xaga binary containing them must themselves be trusted.",
          "If an installed file fails verification, loading stops. Xaga keeps the files for diagnosis and does not silently download replacements. The main window opens with semantic features unavailable, while library write permission stays as it was. Both the model and embedding database must initialize before those features become available.",
        ],
        example:
          "Someone changes an installed model file. Its hash no longer matches the manifest. Xaga refuses to load it, even if the modified model might still produce a vector.",
        questions: [
          {
            prompt:
              "A model file’s hash matches the manifest. What does that establish?",
            answers: [
              {
                text: "The file matches the expected bytes, assuming the manifest and Xaga binary are trusted.",
                feedback:
                  "The comparison checks identity with the chosen artifact, not whether it is harmless.",
                correct: true,
              },
              {
                text: "The model’s publisher has been proven harmless.",
                feedback:
                  "A hash comparison cannot establish intent or safety.",
                correct: false,
              },
              {
                text: "Every future version of that model has been approved.",
                feedback:
                  "A new artifact needs its own expected hashes and review.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "An installed model file fails verification. How should Xaga respond?",
            answers: [
              {
                text: "Stop loading the model, preserve the files, and open with semantic features unavailable.",
                feedback:
                  "This check refuses the changed files; it does not automatically repair them or change library mode.",
                correct: true,
              },
              {
                text: "Delete the files and silently download replacements.",
                feedback: "The model-load verification path does neither.",
                correct: false,
              },
              {
                text: "Accept the file if it still produces a vector.",
                feedback:
                  "Producing output does not replace the integrity check.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Embedding trust chain and runtime failure",
            path: "../architecture/security/embedding_model_trust.md",
          },
          {
            title: "Model verification implementation",
            path: "../../crates/xaga-core/src/embeddings/model.rs",
          },
          {
            title: "Embedding startup UI",
            path: "../../src/providers/EmbeddingStartupProvider.tsx",
          },
        ],
      },
      {
        id: "scale-review",
        level: "Advanced",
        title: "Check large operations as well as individual edits",
        objective:
          "Review large operations for bounded batches, safe retries, and evidence at the intended library size.",
        paragraphs: [
          "Xaga is designed for libraries containing millions of articles. A loop that works for one edit may create excessive database fragments or memory use when it runs during bulk import or recovery. Review both the individual change and the large operation that uses the same code.",
          "Batches and checkpoints limit the work retained for retry. They do not prove a limit on the whole process’s memory: parsing, caches, and the database engine allocate memory too. An index can find matches efficiently while returning millions of matches still costs time and space.",
          "For a change that writes data, trace the source files, locks, per-file queues, priority, and order of saves. Test interruption and retry, including older queued work arriving after newer changes. Record untested assumptions and their code locations instead of presenting them as measured results.",
        ],
        example:
          "For an operation changing many articles, specify the batch size, what is saved before each write, how retry works after each interruption, and how older queued work avoids overwriting newer data. Test those interruptions rather than checking only successful completion.",
        questions: [
          {
            prompt:
              "An operation retains at most 1,024 candidate article IDs at once. What can you conclude about memory?",
            answers: [
              {
                text: "That list is bounded; the rest of the process needs separate measurement.",
                feedback:
                  "Parsing, the database engine, caches, and other data can allocate additional memory.",
                correct: true,
              },
              {
                text: "The whole process has a proven fixed memory ceiling.",
                feedback:
                  "A limit on one list is not a limit on every allocation.",
                correct: false,
              },
              {
                text: "No interruption tests are needed.",
                feedback:
                  "Batch size does not establish correct saving and retry order.",
                correct: false,
              },
            ],
          },
          {
            prompt:
              "How can you verify recovery for an operation that commits several writes?",
            answers: [
              {
                text: "Interrupt it between writes, retry it, and compare saved files with the repaired database.",
                feedback:
                  "That exercises the partial states the implementation must handle.",
                correct: true,
              },
              {
                text: "Only check the success notification after an uninterrupted run.",
                feedback: "That does not test interruption or retry.",
                correct: false,
              },
              {
                text: "Time one article and assume the same rate for two million.",
                feedback:
                  "That neither tests recovery nor establishes performance at the target size.",
                correct: false,
              },
            ],
          },
        ],
        references: [
          {
            title: "Design-scale evidence",
            path: "../architecture/design_scale.md",
          },
          {
            title: "Crash consistency",
            path: "../architecture/crash_consistency.md",
          },
          {
            title: "Least visibility",
            path: "../principles/least_visibility_principle.md",
          },
          {
            title: "Consolidation and DRY",
            path: "../principles/dry.md",
          },
          {
            title: "Development checks",
            path: "../development/release_cycle.md#development-and-local-builds",
          },
        ],
      },
    ],
  },
};
