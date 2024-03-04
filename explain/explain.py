from openai import OpenAI

client = OpenAI()

axioms = """
((e1 is (Receiving LTC))) ∧ Federate(e1) = Federate(e2) ∧ (Tag e1) < (Tag e2)) ∧ (e2 is (Receiving LTC)) ⇒ e1 ≺ e2
(FIRST (((e1 is (Sending STOP_GRN)) ∨ (e1 is (Receiving LTC)) ∨ (((e1 is (Receiving NET)) ∨ (e1 is (Sending TAGGED_MSG))))))) ∧ (Tag e1) = (Tag e2) ∧ ((Tag e1) finite) ∧ (Tag e1) ≠ 0))) ∧ (((e2 is (Sending TAG)) ∨ (e2 is (Sending PTAG))))) ⇒ e1 ≺ e2
(((e1 is (Receiving PORT_ABS)) ∨ (e1 is (Receiving TAGGED_MSG)))) ∧ Federate(e1) = Federate(e2) ∧ (Tag e1) ≤ (Tag e2)) ∧ ((e2 is (Receiving LTC)))) ⇒ e1 ≺ e2
((e1 is (Receiving NET))) ∧ Federate(e1) = Federate(e2) ∧ (Tag e1) ≤ (Tag e2)) ∧ ((e2 is (Receiving LTC))) ∧ (Tag e2) ≠ 0) ⇒ e1 ≺ e2
((e1 is (Receiving LTC))) ∧ Federate(e1) = Federate(e2) ∧ (Tag e1) + (largest delay of a connection from the federate of e1 to the federate of e2) < (Tag e2)) ∧ ((e2 is (Receiving PORT_ABS)) ∨ (e2 is (Receiving TAGGED_MSG))) ⇒ e1 ≺ e2
(FIRST (((e1 is (Sending TAG)) ∨ (e1 is (Sending PTAG)))) ∧ Federate(e1) = Federate(e2) ∧ (Tag e1) + (largest delay of a connection from the federate of e1 to the federate of e2) ≥ (Tag e2))) ∧ (((e2 is (Receiving PORT_ABS)) ∨ (e2 is (Receiving TAGGED_MSG)))) ∧ ¬(Fed e2) has no upstream with delay ≤ (Tag e2)) ⇒ e1 ≺ e2
(((e1 is (Sending PTAG)) ∨ (e1 is (Sending TAG)))) ∧ Federate(e1) = Federate(e2) ∧ (Tag e1) < (Tag e2)) ∧ ((e2 is (Sending PTAG)) ∨ (e2 is (Sending TAG))) ⇒ e1 ≺ e2
((e1 is (Sending PTAG))) ∧ Federate(e1) = Federate(e2) ∧ (Tag e1) ≤ (Tag e2)) ∧ (e2 is (Sending TAG)) ⇒ e1 ≺ e2
(FedwiseFIRST (((e1 is (Receiving LTC))) ∧ (Federate of e1 is upstream of federate of e2 via a zero-delay connection) ∧ (Tag e1) ≥ (Tag e2)) ∨ (((e1 is (Sending TAG)) ∨ (e1 is (Receiving NET)) ∨ (e1 is (Sending STOP_GRN)))) ∧ (Federate of e1 is upstream of federate of e2 via a zero-delay connection) ∧ (Tag e1) ≥ (Tag e2)))) ∧ ((e2 is (Sending TAG))) ∧ (Tag e2) ≠ 0) ⇒ e1 ≺ e2
(FIRST (((e1 is (Sending PTAG))) ∧ (Federate of e1 is upstream of federate of e2 via a zero-delay connection) ∧ (Tag e1) = (Tag e2)) ∨ (((e1 is (Receiving NET)) ∨ (e1 is (Sending STOP_GRN)))) ∧ (Federate(e1) = Federate(e2) ∨ (Federate of e1 is directly upstream of federate of e2)) ∧ (Tag e1) = (Tag e2)))) ∧ ((e2 is (Sending PTAG))) ∧ (Tag e2) ≠ 0) ⇒ e1 ≺ e2
(FIRST ((e1 is (Receiving PORT_ABS))) ∧ (Federate of e1 is upstream of federate of e2 via a zero-delay connection) ∧ (Tag e1) = (Tag e2))) ∧ (e2 is (Sending PORT_ABS)) ⇒ e1 ≺ e2
(FIRST ((e1 is (Receiving TAGGED_MSG))) ∧ (Federate of e1 is directly upstream of federate of e2) ∧ (Tag e1) = (Tag e2))) ∧ (e2 is (Sending TAGGED_MSG)) ⇒ e1 ≺ e2
(((e1 is (Receiving PORT_ABS)) ∨ (e1 is (Receiving TAGGED_MSG)))) ∧ Federate(e1) = Federate(e2) ∧ (Tag e1) ≤ (Tag e2)) ∧ ((e2 is (Receiving LTC))) ⇒ e1 ≺ e2
((e1 is (Receiving FED_ID))) ∧ Federate(e1) = Federate(e2)) ∧ (e2 is (Sending ACK)) ⇒ e1 ≺ e2
((e1 is (Sending ACK))) ∧ Federate(e1) = Federate(e2)) ∧ (e2 is (Receiving TIMESTAMP)) ⇒ e1 ≺ e2
((e1 is (Receiving TIMESTAMP))) ∧ Federate(e1) = Federate(e2)) ∧ (e2 is (Sending TIMESTAMP)) ⇒ e1 ≺ e2
((e1 is (Sending TIMESTAMP))) ∧ Federate(e1) = Federate(e2)) ∧ ((e2 is (Receiving NET))) ∧ ¬(Tag e2) ≠ 0) ⇒ e1 ≺ e2
((e1 is (Receiving TIMESTAMP)))) ∧ ((e2 is (Receiving LTC)) ∨ (e2 is (Receiving PORT_ABS)) ∨ (e2 is (Receiving TAGGED_MSG)) ∨ (e2 is (Sending TAG)) ∨ (e2 is (Sending PTAG)) ∨ (e2 is (Sending PORT_ABS)) ∨ (e2 is (Sending TAGGED_MSG)) ∨ (e2 is (Sending STOP_GRN)) ∨ (e2 is (Sending STOP_REQ)) ∨ (e2 is (Receiving STOP_REQ)) ∨ (e2 is (Receiving STOP_REQ_REP))) ⇒ e1 ≺ e2
""".splitlines()

context = """
A Tag is basically like a time, so it makes sense to add a delay to a Tag.

Throughout, e1 and e2 will denote events occurring in a process called the RTI. Every event involves the RTI either sending a message to a federate, or receiving a message from a federate. So, if Federate(e1) = f, then that means that e1 is an event in which the RTI either sends a message to f or receives a message from f.

Different federates are connected to each other, possibly using multiple connections, and every connection has a delay associated with it.

e1 ≺ e2 means that it is not possible, under any execution of the federated program, for e1 to occur after e2.

Propositions are stated in an S-expression-like format. For example, where we write (f e1), we mean "f of e1".
"""

event_type_prompt = """
The rule is only applicable if the proposition listed below is true. Answer the following two questions:
1. What type of message would need to be sent or received in e1 in order for the rule to potentially be applicable?
2. What type of message would need to be sent or received in e2 in order for the rule to potentially be applicable?
"""

full_explanation_prompt = """
Provide a step-by-step explanation of the conditions that would need to be met in order for the following rule to guarantee that in any execution of the program where e1 and e2 both happen, e1 must occur before e2.
"""

completion = client.chat.completions.create(
    model="gpt-3.5-turbo",
    messages=[
        # {
        #     "role": "system",
        #     "content": "You are a knowledgeable assistant, skilled in explaining mathematical concepts in simple terms.",
        # },
        {
            "role": "system",
            "content": "You are good at getting straight to the point when answering questions.",
        },
        #         {
        #             "role": "system",
        #             "content": """
        # A Tag is basically like a time, so it makes sense to add a delay to a Tag.
        # Throughout, e1 and e2 will denote events occurring in a process called the RTI. Every event involves the RTI either sending a message to a federate, or receiving a message from a federate. So, if Federate(e1) = f, then that means that e1 is an event in which the RTI either sends a message to f or receives a message from f.
        # Different federates are connected to each other, possibly using multiple connections, and every connection has a delay associated with it.
        # e1 ≺ e2 means that it is not possible, under any execution of the federated program, for e1 to occur after e2.
        # Propositions are stated in an S-expression-like format. For example, where we write (f e1), we mean "f of e1".
        #             """,
        #         },
        # {
        #     "role": "user",
        #     "content": """{event_type_prompt}
        #     ((e1 is (Receiving LTC))) ∧ Federate(e1) = Federate(e2) ∧ (Tag e1) < (Tag e2)) ∧ (e2 is (Receiving LTC))""",
        # },
        {
            "role": "user",
            "content": """
            {context}

            {full_explanation_prompt}

            (FIRST (((e1 is (Sending STOP_GRN)) ∨ (e1 is (Receiving LTC)) ∨ (((e1 is (Receiving NET)) ∨ (e1 is (Sending TAGGED_MSG))))))) ∧ (Tag e1) = (Tag e2) ∧ ((Tag e1) finite) ∧ (Tag e1) ≠ 0))) ∧ (((e2 is (Sending TAG)) ∨ (e2 is (Sending PTAG))))) ⇒ e1 ≺ e2""",
        },
    ],
    temperature=0.9,
)

print(completion.choices[0].message)
