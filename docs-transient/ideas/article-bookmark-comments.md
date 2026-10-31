# Article Bookmark Comments

Allow an optional comment alongside an article bookmark's color, analogous to
comments on annotation highlights. This would make a bookmark conceptually a
whole-article annotation rather than a passage annotation.

There is no dedicated article-comment field today. Article `content` holds source
text and `description` holds its summary. Annotation comments live in the parent
article frontmatter as `annotations[].note`, mapped to annotation `content`.
An article comment could also live in frontmatter with an explicit field and
domain mapping.

Questions include whether to use article fields or an annotation without passage
anchors, where to edit and display the comment, when edits are saved, and what
happens to the comment when its color marking is removed.

Relevant code: `src/components/content/AnnotationTooltip.tsx`,
`crates/xaga-core/src/domain.rs`, and `crates/xaga-core/src/md.rs`.
