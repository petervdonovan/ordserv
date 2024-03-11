#!/usr/bin/env python3
import sys
import pickle
from typing import Any, Iterable, Literal, TypedDict
from openai import OpenAI
from openai.types.chat.chat_completion_message_param import ChatCompletionMessageParam

import subprocess
from explain.stringmanip import (
    format_sexpression,
    repair_axiom,
    format_llm_output,
)

import time

client = OpenAI()

Message = TypedDict(
    "Message",
    {
        "kind": Literal["context", "pedantic", "explain", "high-level explain"],
        "message": ChatCompletionMessageParam,
    },
)
Messages = list[Message]

axioms = (
    subprocess.check_output(["cargo", "run", "--example", "print_axioms"], cwd="..")
    .decode(encoding="utf-8")
    .splitlines()
)
axioms = [a for a in axioms if len(a) > 0]

lf_context = """
A Tag is basically like a logical time, so it makes sense to add a logical delay to a Tag.

Throughout, $e_1$ and $e_2$ will denote events occurring in a process called the RTI. Every event involves the RTI either sending a message to a federate, or receiving a message from a federate. So, if Federate($e_1$) = f, then that means that $e_1$ is an event in which the RTI either sends a message to f or receives a message from f.

Federates have ports. Different federates are connected to each other via their ports, possibly using multiple connections, and every connection has a nonnegative logical delay associated with it. If a connection goes from federate $A$ to federate $B$, and the delay associated with that connection is $D$, then that means that when federate A is executing some tag $T$, it is possible for federate A to send a signal that logically reaches federate B at tag $T' := T + D$.

The RTI ensures that if a federate A sends a signal to a federate B that logically reaches federate B at time $T'$, federate B will not execute anything at a tag later than $T'$ before it receives the signal. Furthermore, before federate B receives the signal, federate B can only execute things at the tag $T'$ that are statically guaranteed not to be affected by the signal. When federate B violates these rules by executing something too early, we call that an STP (safe-to-process) violation.

In addition to preventing STP violations, the RTI must prevent deadlocks by allowing federates to proceed forward to logical times when they may have events to process.

The RTI can be described as receiving information from federates, possibly aggregating it into information about the current state of the system, and then propagating the information in such a way that federates can proceed without STP violations.

To ensure that messages arrive in the right order, even messages that conceptually should go from federate to federate go through the RTI. For example, PORT_ABS and TAGGED_MSG messages are sent from an upstream federate to the RTI, and then from the RTI to the downstream federate. However, most messages simply go from a federate to the RTI or from the RTI to a federate, and do not directly correspond to any federate-to-federate communication. For example, federates do not send TAG or PTAG messages; instead, they only receive such messages from the RTI, because it is up to the RTI, not the federates, to control federates' time advancement.

The RTI also coordinates program initialization and shutdown.

A given message is of one of the following types:
1. FED_ID (Federate ID): This is sent from a federate to the RTI during program initialization so that the federate can declare its ID to the RTI.
2. ACK (ACKnowledgement): This message acknowledges receipt of a federate ID.
3. TIMESTAMP: This message is used during initialization to help the federates and the RTI agree on a start time for the main part of the program.
4. NET (Next Event Tag): This message is sent from a federate to the RTI to declare the time of the next event that has been scheduled to occur at that federate so far.
5. PORT_ABS (PORT ABSent): This message communicates that an upstream federate is not going to send a signal that will be received at a particular port of a downstream federate at a particular tag. The federate associated with a PORT_ABS message that the RTI receives is the upstream federate, because that is the federate that the RTI is communicating with; and the federate associated with a PORT_ABS message that the RTI sends is the downstream federate, because then the RTI is communicating to the downstream federate. However, the tag associated with a PORT_ABS message always corresponds to a (logical) tag from the perspective of the downstream federate, both when that message is received by the RTI and when it is forwarded from the RTI to the downstream federate.
6. PTAG (Provisional Tag Advance Grant): This message is sent from the RTI to a federate to communicate that the federate is allowed to proceed forward in logical time up to but not including a particular tag.
7. TAGGED_MSG (Tagged MeSsaGe): This is a signal that goes from a federate to another federate via some connection. The only difference between a TAGGED_MSG and a PORT_ABS message is that a TAGGED_MSG indicates presence of a signal on the connection instead of absence; however, otherwise is very similar to a PORT_ABS message. In particular, just like with a PORT_ABS message, the federate associated with a TAGGED_MSG depends on whether the message is being received or sent by the RTI, whereas its associated tag always corresponds to a (logical) tag from the perspective of the downstream federate.
8. TAG (Tag Advance Grant): This message is sent from the RTI to a federate to communicate that the federate is allowed to proceed forward in logical time up to and including a particular tag.
9. STOP_REQ (STOP REQuest)
10. STOP_REQ_REP (STOP REQuest REPly)
11. STOP_GRN (STOP GRaNted)
12. LTC (Logical Tag Complete): This message is sent from a federate to the RTI to declare that the federate is finished executing a particular tag.

TAGGED_MSG messages that are received from an upstream federate while the upstream federate is executing a tag $T$ are associated with the tag $T + D$ when a federate-to-federate signal on a connection will be received. PORT_ABS messages are similar.
"""

# This means that something is known about $D$, and the RTI receives one of these messages with tag $T + D$, then the RTI can conclude something about the time T that the upstream federate is executing.


syntax_explanation = """
`e_1 ≺ e_2` means that it is not possible, under any physical, real-life execution of the federated program, for `e_1` to occur after `e_2`.

Formulas are stated in an S-expression-like format. For example, where we write `(f e_1)`, we mean "f of $e_1$".

All events are described from the perspective of the RTI. For example, where we write `(e_1 is (Receiving PORT_ABS))`, we mean that $e_1$ is an event in which the RTI receives a PORT_ABS message from a federate; in other words, the federate informed the RTI about when a port is going to be absent. Similarly, where we write `((e_1 is (Sending PORT_ABS)))`, we mean that `e_1` is an event in which the RTI is sending a PORT_ABS to a federate; in other words, the RTI is informing the federate about when one of the ports of that federate is going to be absent.

A formula of the form `(FIRST X)`, where $X$ is some predicate, says that $e_1$ is the first event such that the predicate X is true of $e_1$ and $e_2$.

A formula of the form `(FedwiseFIRST X)`, where `X` is some predicate, says that $e_1$ is the first event occurring at a given federate such that the predicate `X` is true of $e_1$ and $e_2$.

Formulas that use `FIRST` and `FedwiseFIRST` are useful for describing the first event $e_1$ that could possibly cause some other event $e_2$. When we know that $e_2$ must have a cause, but there are multiple events that could have caused $e_2$, we know that the first possible cause of $e_2$ would have had to happen before $e_2$. For example, when we write `(FIRST (X e1))`, where $X$ is some predicate, probably the set of events that make $X$ true is the set of events that could potentially cause some other event $e_2$, and `(FIRST (X e_1))` denotes the first event that could potentially cause $e_2$.
"""

context = f"""
{lf_context}

{syntax_explanation}
"""

subformulas_prompt = """
Break the formula down into its sub-formulas to analyze when its sub-formulas are true. Start by stating when the atomic sub-formulas are true; then, state when the larger sub-formulas that are constructed from the atomic sub-formulas are true, and then state when the sub-formulas constructed from those larger sub-formulas are true, and so on, until you have stated when even the largest sub-formulas are true. Logical operators include the binary operators `∧`, `∨`, and `⇒`, as well as the unary operators `¬`, `FIRST`, and `FedwiseFIRST`. Remember to address the subformula that results from each application of an operator, including the FIRST or FedwiseFIRST operators, and be extra careful when you address the subexpressions that result from FIRST or FedwiseFIRST operators.

Use LaTeX where appropriate. Stop short of discussing the meaning of the whole formula, but do provide a self-contained explanation of when each sub-formula is true, even if that means restating facts about some of the smaller sub-formulas.
"""
# If there are expressions of the form `(FIRST X)` or `(FedwiseFIRST X)` for some predicate $X$, don't forget to explain those subexpressions, too.


# It is guaranteed that the formula given above is always true. That is, for all events $e_1$ and $e_2$ satisfy the antecedent of the implication, $e_1$ is guaranteed to occur before $e_2$ whenever both occur in the execution of a federated program.
whole_formula_prompt = """
Use your analysis of the formula to carefully state what would need to be true about $e_1$ and $e_2$ in order for the formula to guarantee that in any execution of the program where $e_1$ and $e_2$ both happen in the RTI, $e_1$ must occur before $e_2$. Remember, the formula makes this guarantee when $e_1$ and $e_2$ satisfy the antecedent of the implication.

Don't include extraneous information about what you think is important. Don't try to relate the formula to any broader themes. Please just answer the question.
"""

rationale_prompt = """
Use the meaning of the messages and the rules that the RTI and the federates have to follow to explain why we should expect the formula to provide a correct guarantee about the behavior of federated programs. Do not write more than a short paragraph.
"""

# Either state this expectation in terms of a causal relationship, or explain why we need it to be true in order for the federated program to comply with its basic rules of operation.


def start_conversation() -> Messages:
    ret: list[ChatCompletionMessageParam] = [
        {
            "role": "system",
            "content": "You are good at getting straight to the point and focusing on facts.",
        },
        {
            "role": "user",
            "content": context,
        },
    ]
    return [{"kind": "context", "message": message} for message in ret]


def debug_conversation(conversation: list[ChatCompletionMessageParam]) -> str:
    ret = ""
    for message in conversation:
        role = message["role"].replace("\n", "\n  ")
        ret += f"\nROLE: {role}\n"
        content: str | None | Iterable[Any] = message["content"]
        if content is None:
            continue
        if type(content) is str:
            content = content.replace("\n", "\n  ")
            ret += f"MESSAGE: {content}\n"
        else:
            ret += f"MESSAGE: {content}"
    return ret.replace("\n", "\n | ")


def get_subformulas_explanation(messages: Messages, axiom: str) -> tuple[str, Messages]:
    question: Message = {
        "kind": "pedantic",
        "message": {
            "role": "user",
            "content": f"""
Consider the following formula:

{repair_axiom(axiom)}

{subformulas_prompt}
""",
        },
    }
    messages = messages + [question]
    answer = do_query(messages, 0.1, "gpt-3.5-turbo-0125")
    return (
        answer,
        messages
        + [
            {"kind": "pedantic", "message": {"role": "assistant", "content": answer}},
        ],
    )


def get_whole_formula_explanation(
    messages: Messages, _axiom: str
) -> tuple[str, Messages]:
    question: Message = {
        "kind": "explain",
        "message": {
            "role": "user",
            "content": f"""
{whole_formula_prompt}
""",
        },
    }
    messages = messages + [question]
    answer = do_query(messages, 0.3, "gpt-3.5-turbo-0125")
    return (
        answer,
        messages
        + [
            {
                "kind": "explain",
                "message": {"role": "assistant", "content": answer},
            }
        ],
    )


def get_rationale_explanation(messages: Messages, _axiom: str) -> tuple[str, Messages]:
    question: Message = {
        "kind": "high-level explain",
        "message": {
            "role": "user",
            "content": f"""
{rationale_prompt}
""",
        },
    }
    messages = messages + [question]
    answer = do_query(messages, 0.3, "gpt-4-0125-preview")
    return (
        answer,
        messages
        + [
            {
                "kind": "explain",
                "message": {"role": "assistant", "content": answer},
            }
        ],
    )


def do_query(
    messages: Messages,
    temperature: float,
    model: Literal["gpt-4-0125-preview", "gpt-3.5-turbo-0125"],
) -> str:
    global raw_answers
    llm_messages = [message["message"] for message in messages]
    answer: str | None
    if dry_run:
        print(debug_conversation(llm_messages))  # DEBUG
        answer = "<LLM answer here>"
    else:
        reply = client.chat.completions.create(
            model=model,
            messages=llm_messages,
            temperature=temperature,  # 0.3 has been recommended for code comment generation, which is kind of like what we are doing here. However, it is not clear how much performance actually depends on temperature for this use case.
        ).choices[0]
        answer = reply.message.content
        if answer is None:
            raise RuntimeError("did not get an answer from the LLM")
    answer = format_llm_output(answer)
    raw_answers += [answer]

    return answer


def print_time():
    global t0
    print(f"\n\n(This answer was generated in {round(time.time() - t0)} seconds.)\n")
    t0 = time.time()


raw_answers: list[str] = []

dry_run = len(sys.argv) > 1
if dry_run:
    print("\n\ndoing a dry run because an argument was passed.\n\n\n")
# axioms_sorted = sorted(axioms, key=lambda x: -len(x))


print("## Background: LF Federated Execution")
print()
print(lf_context)
print()
print("## Preliminary Syntax Explanation")
print()
print(syntax_explanation)
conversation: Messages = start_conversation()
t0 = time.time()
for i, axiom in enumerate(axioms[5:6], start=1):
    print(f"## Formula {i}\n")
    print(f"Formula {i} states:\n`{format_sexpression(axiom)}`\n")
    print(f"### In-depth syntactic explanation")
    answer, conversation = get_subformulas_explanation(conversation, axiom)
    print(answer)
    print_time()
    print(f"### Summary of the meaning of formula {i}")
    answer, conversation = get_whole_formula_explanation(conversation, axiom)
    print(answer)
    print_time()
    print(f"### High-level justification")
    answer, conversation = get_rationale_explanation(conversation, axiom)
    print(answer)
    print_time()
    # TODO
    print("\n")

with open("raw_answers_temp.pkl", "wb") as f:
    pickle.dump(raw_answers, f, pickle.HIGHEST_PROTOCOL)
