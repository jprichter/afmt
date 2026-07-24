# Name-path chain formatting

afmt keeps the dot glued to a pure dotted-identifier path, such as
`System.Assert` or `Schema.SObjectType`, because Apex rejects whitespace before
the dot when the path is parsed as a type or namespace reference. This applies
to every dot in the path, including dots before a later member.

The formatter uses only the syntax tree for this rule. It does not resolve
identifiers against a symbol table or Salesforce type list, so a path beginning
with a variable is glued as well. Value expressions are not treated as name
paths: method calls, array access, and SOQL expressions can break before a
following member when their chain group overflows, while object creation,
literals, parenthesized expressions, and `this` or `super` retain their own
existing expression break opportunities.

This is a canonical reflow rule: existing whitespace before a name-path dot is
repaired rather than preserved. The battle-test script includes a syntactic
check for any remaining line-start dot whose preceding nonblank line is a pure
dotted-identifier path.
