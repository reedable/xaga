# Estimated reading time

Estimate how long an article takes to read from its available text, helping the
user choose reading that fits the time they have.

A possible starting point is readable word count divided by an assumed reading
speed. For illustration, 1,000 words at 200 words per minute yields five minutes;
this is an assumption, not a measurement of the reader. Exclude markup and page
clutter from the count.

Word count is simple to explain but does not account for time spent on images,
code, equations, or unfamiliar material. Languages without space-separated words
need an appropriate segmentation or character-based approach. Partial article
text also limits what the estimate represents.

Reading speed, language handling, rounding, display location, and whether users
can adjust the estimate remain open choices. A fixed rate would keep the feature
simple; an adjustable rate could better reflect individual readers but adds
configuration.
