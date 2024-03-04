## Preliminary syntax explanation

e1 ≺ e2 means that it is not possible, under any execution of the federated program, for e1 to occur after e2.

Propositions are stated in an S-expression-like format. For example, where we write (f e1), we mean "f of e1".

An expression of the form (FIRST X), where X is some proposition, says that e1 is the first event e1 such that the proposition X is true of e1 and e2.

An expression of the form (FedwiseFIRST X), where X is some proposition, says that e1 is the first event e1 occurring at a given federate such that the proposition X is true of e1 and e2.

## Sentence 1

Sentence 1 states:
((((e1 is (Receiving LTC))) ∧ (Federate(e1) = Federate(e2)) ∧ ((Tag e1) < (Tag e2))) ∧ ((e2 is (Receiving LTC)))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 1 will make a guarantee about two events, e1 and e2:

## Sentence 2

Sentence 2 states:
(((FIRST (((e1 is (Sending STOP_GRN))) ∨ ((e1 is (Receiving LTC))) ∨ ((((e1 is (Receiving NET))) ∨ ((e1 is (Sending TAGGED_MSG)))))) ∧ ((Tag e1) = (Tag e2)) ∧ (((Tag e1) finite) ∧ ((Tag e1) ≠ 0)))) ∧ ((((e2 is (Sending TAG))) ∨ ((e2 is (Sending PTAG)))))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 2 will make a guarantee about two events, e1 and e2:

## Sentence 3

Sentence 3 states:
(((((e1 is (Receiving PORT_ABS))) ∨ ((e1 is (Receiving TAGGED_MSG)))) ∧ (Federate(e1) = Federate(e2)) ∧ ((Tag e1) ≤ (Tag e2))) ∧ (((e2 is (Receiving LTC))))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 3 will make a guarantee about two events, e1 and e2:

## Sentence 4

Sentence 4 states:
((((e1 is (Receiving NET))) ∧ (Federate(e1) = Federate(e2)) ∧ ((Tag e1) ≤ (Tag e2))) ∧ (((e2 is (Receiving LTC))) ∧ ((Tag e2) ≠ 0))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 4 will make a guarantee about two events, e1 and e2:

## Sentence 5

Sentence 5 states:
((((e1 is (Receiving LTC))) ∧ (Federate(e1) = Federate(e2)) ∧ ((Tag e1) + (largest delay of a connection going out of the federate of e1) < (Tag e2))) ∧ (((e2 is (Receiving PORT_ABS))) ∨ ((e2 is (Receiving TAGGED_MSG))))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 5 will make a guarantee about two events, e1 and e2:

## Sentence 6

Sentence 6 states:
(((FIRST (((e1 is (Sending TAG))) ∨ ((e1 is (Sending PTAG)))) ∧ (Federate(e1) = Federate(e2)) ∧ ((Tag e1) + (largest delay of a connection going out of the federate of e1) ≥ (Tag e2)))) ∧ ((((e2 is (Receiving PORT_ABS))) ∨ ((e2 is (Receiving TAGGED_MSG)))) ∧ (¬((Fed e2) has no upstream with delay ≤ (Tag e2))))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 6 will make a guarantee about two events, e1 and e2:

## Sentence 7

Sentence 7 states:
(((((e1 is (Sending PTAG))) ∨ ((e1 is (Sending TAG)))) ∧ (Federate(e1) = Federate(e2)) ∧ ((Tag e1) < (Tag e2))) ∧ (((e2 is (Sending PTAG))) ∨ ((e2 is (Sending TAG))))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 7 will make a guarantee about two events, e1 and e2:

## Sentence 8

Sentence 8 states:
((((e1 is (Sending PTAG))) ∧ (Federate(e1) = Federate(e2)) ∧ ((Tag e1) ≤ (Tag e2))) ∧ ((e2 is (Sending TAG)))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 8 will make a guarantee about two events, e1 and e2:

## Sentence 9

Sentence 9 states:
(((FedwiseFIRST (((e1 is (Receiving LTC))) ∧ ((Federate of e1 is upstream of federate of e2 via a zero-delay connection)) ∧ ((Tag e1) ≥ (Tag e2))) ∨ ((((e1 is (Sending TAG))) ∨ ((e1 is (Receiving NET))) ∨ ((e1 is (Sending STOP_GRN)))) ∧ ((Federate of e1 is upstream of federate of e2 via a zero-delay connection)) ∧ ((Tag e1) ≥ (Tag e2))))) ∧ (((e2 is (Sending TAG))) ∧ ((Tag e2) ≠ 0))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 9 will make a guarantee about two events, e1 and e2:

## Sentence 10

Sentence 10 states:
(((FIRST (((e1 is (Sending PTAG))) ∧ ((Federate of e1 is upstream of federate of e2 via a zero-delay connection)) ∧ ((Tag e1) = (Tag e2))) ∨ ((((e1 is (Receiving NET))) ∨ ((e1 is (Sending STOP_GRN)))) ∧ ((Federate(e1) = Federate(e2)) ∨ ((Federate of e1 is directly upstream of federate of e2))) ∧ ((Tag e1) = (Tag e2))))) ∧ (((e2 is (Sending PTAG))) ∧ ((Tag e2) ≠ 0))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 10 will make a guarantee about two events, e1 and e2:

## Sentence 11

Sentence 11 states:
(((FIRST ((e1 is (Receiving PORT_ABS))) ∧ ((Federate of e1 is upstream of federate of e2 via a zero-delay connection)) ∧ ((Tag e1) = (Tag e2)))) ∧ ((e2 is (Sending PORT_ABS)))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 11 will make a guarantee about two events, e1 and e2:

## Sentence 12

Sentence 12 states:
(((FIRST ((e1 is (Receiving TAGGED_MSG))) ∧ ((Federate of e1 is directly upstream of federate of e2)) ∧ ((Tag e1) = (Tag e2)))) ∧ ((e2 is (Sending TAGGED_MSG)))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 12 will make a guarantee about two events, e1 and e2:

## Sentence 13

Sentence 13 states:
(((((e1 is (Receiving PORT_ABS))) ∨ ((e1 is (Receiving TAGGED_MSG)))) ∧ (Federate(e1) = Federate(e2)) ∧ ((Tag e1) ≤ (Tag e2))) ∧ (((e2 is (Receiving LTC))))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 13 will make a guarantee about two events, e1 and e2:

## Sentence 14

Sentence 14 states:
((((e1 is (Receiving FED_ID))) ∧ (Federate(e1) = Federate(e2))) ∧ ((e2 is (Sending ACK)))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 14 will make a guarantee about two events, e1 and e2:

## Sentence 15

Sentence 15 states:
((((e1 is (Sending ACK))) ∧ (Federate(e1) = Federate(e2))) ∧ ((e2 is (Receiving TIMESTAMP)))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 15 will make a guarantee about two events, e1 and e2:

## Sentence 16

Sentence 16 states:
((((e1 is (Receiving TIMESTAMP))) ∧ (Federate(e1) = Federate(e2))) ∧ ((e2 is (Sending TIMESTAMP)))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 16 will make a guarantee about two events, e1 and e2:

## Sentence 17

Sentence 17 states:
((((e1 is (Sending TIMESTAMP))) ∧ (Federate(e1) = Federate(e2))) ∧ (((e2 is (Receiving NET))) ∧ (¬((Tag e2) ≠ 0)))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 17 will make a guarantee about two events, e1 and e2:

## Sentence 18

Sentence 18 states:
((((e1 is (Receiving TIMESTAMP)))) ∧ (((e2 is (Receiving LTC))) ∨ ((e2 is (Receiving PORT_ABS))) ∨ ((e2 is (Receiving TAGGED_MSG))) ∨ ((e2 is (Sending TAG))) ∨ ((e2 is (Sending PTAG))) ∨ ((e2 is (Sending PORT_ABS))) ∨ ((e2 is (Sending TAGGED_MSG))) ∨ ((e2 is (Sending STOP_GRN))) ∨ ((e2 is (Sending STOP_REQ))) ∨ ((e2 is (Receiving STOP_REQ))) ∨ ((e2 is (Receiving STOP_REQ_REP))))) ⇒ (e1 ≺ e2)

Here is an LLM's explanation of when proposition 18 will make a guarantee about two events, e1 and e2:
