#!/usr/bin/env python3

from openai import OpenAI

import subprocess
import re

client = OpenAI()

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

The RTI ensures that if a federate A sends a signal to a federate B that logically reaches federate B at time $T'$, federate B will not execute anything at a tag later than $T'$ before it receives the signal. Furthermore, before federate B receives the signal, federate B can only execute things at the tag $T'$ that are statically guaranteed not to be affected by the signal. When federate B violates these rules by executing something too early, we call that an STP violation.

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
`e_1 ≺ e_2` means that it is not possible, under any execution of the federated program, for `e_1` to occur after `e_2`.

Sentences are stated in an S-expression-like format. For example, where we write `(f e_1)`, we mean "f of $e_1$".

All events are described from the perspective of the RTI. For example, where we write `(e_1 is (Receiving PORT_ABS))`, we mean that $e_1$ is an event in which the RTI receives a PORT_ABS message from a federate; in other words, the federate informed the RTI about when a port is going to be absent. Similarly, where we write `((e_1 is (Sending PORT_ABS)))`, we mean that `e_1` is an event in which the RTI is sending a PORT_ABS to a federate; in other words, the RTI is informing the federate about when one of the ports of that federate is going to be absent.

An expression of the form `(FIRST X)`, where $X$ is some predicate, says that $e_1$ is the first event such that the predicate X is true of $e_1$ and $e_2$.

An expression of the form `(FedwiseFIRST X)`, where `X` is some predicate, says that $e_1$ is the first event occurring at a given federate such that the predicate `X` is true of $e_1$ and $e_2$.

Expressions like that use `FIRST` and `FedwiseFIRST` are useful for describing the first event $e_1$ that could possibly cause some other event $e_2$. When we know that $e_2$ must have a cause, but there are multiple events that could have caused $e_2$, we know that the first possible cause of $e_2$ would have had to happen before $e_2$. For example, when we write `(FIRST (X e1))`, where $X$ is some predicate, probably the set of events that make $X$ true is the set of events that could potentially cause some other event $e_2$, and `(FIRST (X e_1))` denotes the first event that could potentially cause $e_2$.
"""

context = f"""
{lf_context}

{syntax_explanation}
"""

full_explanation_prompt = """
In the following tasks, use some LaTeX where appropriate.

Recursively list out the subexpressions that appear in the sentence, and while you do that, state the conditions under which the subexpression is true. Focus on the most complex subexpressions, not just the smallest ones. If there are expressions of the form `(FIRST X)` or `(FedwiseFIRST X)` for some predicate $X$, don't forget to explain those subexpressions, too.

Then, use your analysis of the subexpressions to carefully state what would need to be true in order for the sentence to guarantee that in any execution of the program where $e_1$ and $e_2$ both happen in the RTI, $e_1$ must occur before $e_2$. Note that the sentence makes this guarantee whenever its largest subexpression (the expression preceding the ⇒ symbol) is true.

Then, use the meaning of the messages and the rules that the RTI and the federates have to follow to briefly describe why we should expect the sentence to provide a correct guarantee about the behavior of federated programs.
"""

# Either state this expectation in terms of a causal relationship, or explain why we need it to be true in order for the federated program to comply with its basic rules of operation.


def get_explanation(axiom):
    main_question = f"""

Consider the following sentence:

{repair_axiom(axiom)}

{full_explanation_prompt}
"""
    print(main_question)
    return client.chat.completions.create(
        model="gpt-4-0125-preview",
        messages=[
            {
                "role": "system",
                "content": "You are good at getting straight to the point.",
            },
            {
                "role": "user",
                "content": context,
            },
            {
                "role": "user",
                "content": main_question,
            },
        ],
        temperature=0.3,  # 0.3 has been recommended for code comment generation, which is kind of like what we are doing here. However, it is not clear how much performance actually depends on temperature for this use case.
    )


# axioms_sorted = sorted(axioms, key=lambda x: -len(x))


def repair_axiom(axiom):
    return axiom.replace("e1", "e_1").replace("e2", "e_2")


def repair_latex(text):
    # Define a regular expression pattern to find \text{} macros
    text_macro_pattern = re.compile(r"\\text\{([^{}]*)\}")

    # Find all matches of \text{} macros in the input text
    matches = text_macro_pattern.finditer(text)

    # Iterate through matches and escape underscores inside \text{} macros
    for match in matches:
        text_inside_macro = match.group(1)
        repaired_text_inside_macro = text_inside_macro.replace("_", r"\_").replace(
            r"\\_", r"\_"
        )
        repaired_macro = f"\\text{{{repaired_text_inside_macro}}}"
        text = text.replace(match.group(0), repaired_macro)

    return text


def format_llm_output(s):
    ret = "  " + s.replace("\n", "\n  ").replace("\(", "$").replace("\)", "$").replace(
        "\[", "$"
    ).replace("\]", "$")
    return repair_latex(ret)


print("## Background: LF Federated Execution")
print()
print(lf_context)
print()
print("## Preliminary Syntax Explanation")
print()
print(syntax_explanation)
for i, axiom in enumerate(axioms[5:6], start=1):
    print(f"## Sentence {i}\n")
    print(f"Sentence {i} states:\n`{axiom}`\n")
    print(
        f"Here is an LLM's explanation of when sentence {i} will make a guarantee about two events, $e_1$ and $e_2$:\n"
    )
    print(format_llm_output(get_explanation(axiom).choices[0].message.content))
    print("\n")
