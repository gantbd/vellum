---------------------------- MODULE Execution ----------------------------
EXTENDS Naturals, FiniteSets, TLC

\* Design abstraction of ONE admitted execution, not the complete editor.
\* Verbs represent operation families, not the final public verb catalog.
\* Docs are abstract atomic participants; booleans abstract current context.
CONSTANTS Docs, MaxFuel, RecheckContext, HonorFuel
ASSUME /\ Docs # {}
       /\ IsFiniteSet(Docs)
       /\ MaxFuel \in Nat \ {0}
       /\ RecheckContext \in BOOLEAN
       /\ HonorFuel \in BOOLEAN

Verbs == {"Edit", "Annotate", "Undo", "Save", "Read"}
InputVerbs == Verbs \cup {"UnsupportedVerb"}
Events == {"Admit", "Prepare", "Commit", "Dispatch", "Release",
           "ReceiptOK", "ReceiptFailed", "ReceiptQuiescent", "Revoke", "DependencyChanged",
           "Cancel", "Deadline", "BadInput", "Tick", "Surprise"}
Terminal == {"Done", "Rejected", "Stopped", "Fault"}
Phases == {"New", "Admitted", "Prepared", "Waiting"} \cup Terminal
Facts == {"OK", "Failed", "Quiescent"}
Before == [d \in Docs |-> 0]
After == [d \in Docs |-> 1]

StateSpace == [phase : Phases, verb : InputVerbs, fuel : 0..MaxFuel,
               granted : BOOLEAN, current : BOOLEAN, applied : BOOLEAN,
               dispatched : BOOLEAN, released : BOOLEAN, pulse : BOOLEAN,
               contents : [Docs -> {0, 1}], evidence : SUBSET Facts]
Halted(s) == s.phase \in Terminal
Receipt(e) == e \in {"ReceiptOK", "ReceiptFailed", "ReceiptQuiescent"}
ReceiptFact(e) == IF e = "ReceiptOK" THEN "OK" ELSE IF e="ReceiptFailed" THEN "Failed" ELSE "Quiescent"
CanApply(s) == IF RecheckContext THEN s.granted /\ s.current ELSE TRUE
Rank(s) == IF Halted(s) THEN 0 ELSE s.fuel

\* Total decision over the declared domain: all unmatched admitted events
\* fault the execution; invalid external input/early receipts are distinct.
Decision(s, e) ==
  IF Halted(s)
    THEN IF s.dispatched /\ Receipt(e)
           THEN [s EXCEPT !.evidence = @ \cup {ReceiptFact(e)}]
           ELSE s
  ELSE IF e = "Revoke" THEN [s EXCEPT !.granted = FALSE]
  ELSE IF e = "DependencyChanged" THEN [s EXCEPT !.current = FALSE]
  ELSE IF e \in {"Cancel", "Deadline"} THEN [s EXCEPT !.phase = "Stopped"]
  ELSE IF e = "BadInput"
    THEN IF s.phase = "New" THEN [s EXCEPT !.phase = "Rejected"] ELSE s
  ELSE IF e = "Tick" THEN s
  ELSE IF Receipt(e)
    THEN IF s.dispatched
           THEN [s EXCEPT !.phase = "Done",
                         !.evidence = @ \cup {ReceiptFact(e)}]
           ELSE s  \* rejected early/unrelated receipt, no invented evidence
  ELSE IF s.phase = "New" /\ e = "Admit"
    THEN [s EXCEPT !.phase = IF s.verb \in Verbs THEN "Admitted" ELSE "Rejected"]
  ELSE IF s.phase = "Admitted" /\ e = "Prepare"
    THEN [s EXCEPT !.phase = IF s.granted /\ s.current THEN "Prepared" ELSE "Rejected"]
  ELSE IF s.phase = "Prepared" /\ e = "Commit" /\
          s.verb \in {"Edit", "Annotate", "Undo"}
    THEN IF CanApply(s)
           THEN [s EXCEPT !.phase = "Done", !.applied = TRUE, !.contents = After]
           ELSE [s EXCEPT !.phase = "Rejected"]
  ELSE IF s.phase = "Prepared" /\ e = "Dispatch" /\ s.verb = "Save"
    THEN IF CanApply(s)
           THEN [s EXCEPT !.phase = "Waiting", !.dispatched = TRUE]
           ELSE [s EXCEPT !.phase = "Rejected"]
  ELSE IF s.phase = "Prepared" /\ e = "Release" /\ s.verb = "Read"
    THEN IF CanApply(s)
           THEN [s EXCEPT !.phase = "Done", !.released = TRUE]
           ELSE [s EXCEPT !.phase = "Rejected"]
  ELSE [s EXCEPT !.phase = "Fault"]

Step(s, e) ==
  IF Halted(s) THEN Decision(s, e)
  ELSE IF s.fuel = 0 THEN [s EXCEPT !.phase = "Stopped"]
  ELSE LET candidate == Decision(s, e)
           charged == [candidate EXCEPT
                         !.fuel = IF HonorFuel THEN s.fuel - 1 ELSE s.fuel,
                         !.pulse = ~s.pulse]
       IN IF charged.fuel = 0 /\ ~Halted(charged)
            THEN [charged EXCEPT !.phase = "Stopped"]
            ELSE charged

VARIABLE state
Init == \E v \in InputVerbs:
  state = [phase |-> "New", verb |-> v, fuel |-> MaxFuel,
           granted |-> TRUE, current |-> TRUE, applied |-> FALSE,
           dispatched |-> FALSE, released |-> FALSE, pulse |-> FALSE,
           contents |-> Before, evidence |-> {}]
Next == \E e \in Events: state' = Step(state, e)
\* Weak fairness models service of local execution/deadline events. It does
\* not assert external write success, wall-clock timing or native preemption.
Spec == Init /\ [][Next]_state /\ WF_state(Next)

TypeOK == state \in StateSpace
AtomicPublication == state.contents \in {Before, After}
NoRejectedPublication == state.phase = "Rejected" => ~state.applied
PublicationMatchesContent == (state.applied <=> state.contents = After)
NoForgedEvidence == ~state.dispatched => state.evidence = {}
UnsupportedNeverExecutes == state.verb \notin Verbs =>
  ~state.applied /\ ~state.dispatched /\ ~state.released
ActiveHasFuel == ~Halted(state) => state.fuel > 0
TotalOnReachable == \A e \in Events: Step(state, e) \in StateSpace
ConflictingEvidence == {"OK", "Failed"} \subseteq state.evidence
UnknownWrite == state.dispatched /\ state.evidence = {}

PublishAuthorized ==
  [][((~state.applied /\ state'.applied) => state.granted /\ state.current)]_state
ReleaseAuthorized ==
  [][((~state.released /\ state'.released) => state.granted /\ state.current)]_state
DispatchAuthorized ==
  [][((~state.dispatched /\ state'.dispatched) => state.granted /\ state.current)]_state
EvidenceMonotone == [][state.evidence \subseteq state'.evidence]_state
RankDecreases == [][(~Halted(state) => Rank(state') < Rank(state))]_state
TerminalControlStable == [][(Halted(state) => state'.phase = state.phase)]_state
Termination == <>Halted(state)
=============================================================================
