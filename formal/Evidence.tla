----------------------------- MODULE Evidence -----------------------------
EXTENDS Naturals, FiniteSets
CONSTANT Facts
ASSUME IsFiniteSet(Facts)

Domain == SUBSET Facts
Bottom == {}
Top == Facts
Leq(a, b) == a \subseteq b
Join(a, b) == a \cup b
Meet(a, b) == a \cap b

\* Facts are immutable observations about ONE fixed effect/attempt/scope.
\* Contradictory receipts coexist as evidence; top does not mean success.
LatticeLaws ==
  /\ \A a \in Domain:
       /\ Join(a, Bottom) = a
       /\ Meet(a, Top) = a
       /\ Join(a, a) = a
       /\ Meet(a, a) = a
       /\ Leq(Bottom, a) /\ Leq(a, Top)
  /\ \A a, b \in Domain:
       /\ Join(a, b) \in Domain /\ Meet(a, b) \in Domain
       /\ Join(a, b) = Join(b, a)
       /\ Meet(a, b) = Meet(b, a)
       /\ Join(a, Meet(a, b)) = a
       /\ Meet(a, Join(a, b)) = a
       /\ Leq(a, Join(a, b)) /\ Leq(b, Join(a, b))
       /\ Leq(Meet(a, b), a) /\ Leq(Meet(a, b), b)
  /\ \A a, b, c \in Domain:
       /\ Join(Join(a, b), c) = Join(a, Join(b, c))
       /\ Meet(Meet(a, b), c) = Meet(a, Meet(b, c))
       /\ ((Leq(a, c) /\ Leq(b, c)) => Leq(Join(a, b), c))
       /\ ((Leq(c, a) /\ Leq(c, b)) => Leq(c, Meet(a, b)))
       /\ (Leq(a, b) => Leq(Join(a, c), Join(b, c)))

VARIABLE token
Init == token = 0
Next == UNCHANGED token
Spec == Init /\ [][Next]_token
=============================================================================
