# Writing Style

Shared guidance for Xaga documentation, training, the User Guide, and UI copy.
Use consistent terminology across these surfaces, adapting detail to the audience.

## Audience and content

- **User Guide and product training:** address the reader as “you.” Explain
  workflows through actions and visible outcomes, using actual UI labels. Start
  with the situation and task; include implementation details only when they
  help the reader act or understand a consequence.
- **Interface and engineering training:** address the trainee as “you” and the
  person whose experience they design as “the user.” Introduce implementation
  terms before relying on them and connect explanations to their effects.
- **Technical documentation:** name components, APIs, files, and saved values
  precisely. Explain contracts, reasons, exceptions, and limits needed to
  implement or review the system. Use “the user” for product behavior and “you”
  for instructions to the reader.

Keep instructions concise and ordered around the task. Introduce example names
and objects before using them. Use screenshots where they clarify an action or
outcome, with meaningful alternative text; recordings need equivalent written
steps. Verify behavior against code and the app, distinguishing implemented
behavior from proposals and target designs.

Quote current UI labels exactly. If a label conflicts with this guide, raise an
explicit UI copy correction rather than inventing a different label in instructions.

## Shared language

State the rule or behavior directly. Name the component, file, value, or user
operation involved. Explain established technical terms when their meaning is
not clear from context. Use inline code formatting for identifiers and paths.

Avoid invented technical labels, extended metaphors, slogans, and abstract
phrases that require the person reading the document to reconstruct the operation. For example,
“the command passes the saved color to the indexing job” explains more than
“committed intent crosses the boundary.” Describe the actual check or saved
state rather than giving it an impressive name.

Keep requirements, exceptions, measurements, and known limits explicit when
simplifying prose. Clearer language must not silently change a design decision
or make a security claim stronger than the available evidence.

## Terminology

Use a stable name for each person, product part, and operation. Do not alternate
synonyms for variety when they could suggest different things.

| Meaning                                                 | Term in prose                                                              |
| ------------------------------------------------------- | -------------------------------------------------------------------------- |
| Person using the product                                | user                                                                       |
| The application                                         | Xaga                                                                       |
| UI area displaying article content                      | reading pane                                                               |
| Assistive technology that announces content             | screen reader                                                              |
| Software sending HTTP requests                          | HTTP client                                                                |
| One running application process                         | Xaga process                                                               |
| Saved web article                                       | article; use `Entry` or `Article` when discussing those exact code types   |
| Highlighted passage and optional comment                | annotation; use `Annotation` for the code type                             |
| Inspector tab for choosing Topics                       | Topics view                                                                |
| External website where an article was published         | website                                                                    |
| List of saved articles sharing a website hostname       | articles from that website                                                 |
| Navigation section listing those websites               | Sites section                                                              |
| Domain object and sidebar component for that navigation | `Site` data type and `SiteList` component, respectively                    |
| Fixed-schema record in a database table                 | database row; do not use “row” for an article in a UI list                 |
| Whether an article is read or unread                    | read status; keep exact field names such as `is_unread` in code formatting |

Use visible UI labels when explaining controls and views. Preserve code names,
file paths, and quoted output exactly, marking identifiers as code. If a code
name differs from the prose term, explain the relationship on first use.

Distinguish operations as well as nouns. Removing a shortcut does not delete its
destination. Removing a tag from one article does not delete the tag definition.
Saved is the UI view containing bookmarked articles and annotations, not another
name for bookmarks alone. Specify the object changed rather than saying only
“remove it,” “update the state,” or “change the target.”

## UI copy

UI copy includes labels, buttons, tooltips, empty states, progress, confirmations,
and errors. Follow the shared terminology and use the same wording for the same
action. These rules guide new and revised copy; they do not imply that every
existing label already conforms.

- Use sentence case for new labels and headings, preserving product names and
  established visible names such as Topics and Saved. Short labels and buttons
  need no terminal period; punctuate complete explanatory sentences normally.
- Name actions with specific verbs and objects where context does not identify
  the object: **Remove shortcut**, rather than **OK**. Keep labels short without
  obscuring their effect. Tooltips may explain an action but must not be its only
  accessible name or the only source of an essential consequence.
- Distinguish navigation, removal from a collection, and deletion of saved data.
  Confirmations state the affected object and consequence; the action button
  names the operation. Explain security consequences before the user decides.
- Empty states say what is empty and offer an available next step. Distinguish
  empty results, loading, and unavailable content. Unavailable does not mean deleted.
- Progress names the operation in progress; claim completion only when success
  is known. Communicate status independently of animation, following the
  [motion convention](motion.md).
- Errors explain what failed, what is known about saved changes, and an available
  next step. Describe observed failure without guessing its cause or blaming the
  user or website. “We can’t find this topic” does not claim it was deleted.
  Keep diagnostics out of primary copy unless they help the user decide what to do.

## Related conventions

- [Documentation](documentation.md) defines organization, task lifecycle, and formats.
- [Training maintenance](../training/README.md#writing-lessons) defines lesson and quiz guidance.
- [Semantic vocabulary](vocabulary/semantic_vocabulary.md) and
  [conventional vocabulary](vocabulary/conventional_vocabulary.md) define symbol naming.
