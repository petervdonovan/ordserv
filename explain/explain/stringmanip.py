import re


def repair_axiom(axiom: str) -> str:
    return axiom.replace("e1", "e_1").replace("e2", "e_2")


def repair_latex(text: str) -> str:
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


def format_llm_output(s: str, min_heading_depth=4) -> str:
    ret = "  " + s.replace("\n", "\n  ").replace("\(", "$").replace("\)", "$").replace(
        "\[", "$"
    ).replace("\]", "$")
    for depth in range(1, min_heading_depth):
        ret = ret.replace(
            "\n" + "#" * depth + " ", "\n" + "#" * min_heading_depth + " "
        )
    return repair_latex(ret)


def find_end(tokens: list[str], start: int) -> int:
    i = start
    depth = 0
    while depth >= 0:
        if tokens[i] == ")":
            depth -= 1
        elif tokens[i] == "(":
            depth += 1
        i += 1
    return i


def can_fit(tokens: list[str], start: int, depth: int) -> bool:
    end = find_end(tokens, start)
    length = depth * 2 + sum(len(tokens[i]) for i in range(start, end))
    return length < 80


def wrap_as_needed(s: str) -> str:
    ret = ""
    for line in s.split("\n"):
        if len(line) < 80:
            ret += line  # + " " * (80 - len(line)) + "|"
        else:
            ret += wrap_text(
                line,
            )
        ret += "\n"
    return ret


def word_wrap(input_string, column_width=80):
    words = input_string.split()
    lines = []
    current_line = ""

    for word in words:
        if len(current_line) + len(word) + 1 <= column_width:
            # Add word to the current line
            current_line += word + " "
        else:
            # Start a new line
            lines.append(current_line.rstrip())
            current_line = word + " "

    # Add the last line
    lines.append(current_line.rstrip())

    # Join the lines to form the wrapped text
    wrapped_text = "\n".join(lines)

    return wrapped_text


def wrap_text(input_string: str, column_width=80):
    indentation = len(input_string) - len(input_string.strip())
    space_remaining = column_width - indentation
    line = word_wrap(input_string, space_remaining)
    return " " * indentation + line.replace("\n", "\n" + " " * indentation)


def format_sexpression(input_str: str) -> str:
    tokens = input_str.replace("(", " ( ").replace(")", " ) ").split()
    formatted_str = ""
    depth = 0
    cram_on_one_line = False
    cram_on_one_line_depth: int | None = None

    for i, token in enumerate(tokens):
        if token == "(":
            j = i + 1
            if not cram_on_one_line:
                cram_on_one_line = can_fit(tokens, i + 1, depth)
                cram_on_one_line_depth = depth
            # while tokens[j] not in ["(", ")"]:
            #     j += 1
            # cram_on_one_line = tokens[j] == ")"
            if formatted_str and formatted_str[-1] == "(":
                formatted_str += "("
            elif cram_on_one_line:
                formatted_str += " ("
            else:
                formatted_str += "\n" + "  " * depth + "("
            depth += 1
        elif token == ")":
            depth -= 1
            if cram_on_one_line:
                formatted_str += ")"
            else:
                formatted_str += ")"
            if depth == cram_on_one_line_depth:
                cram_on_one_line_depth = None
                cram_on_one_line = False
        elif formatted_str and formatted_str[-1] == "(":
            formatted_str += token
        elif cram_on_one_line:
            formatted_str += " " + token
        else:
            formatted_str += "\n" + "  " * depth + token

    return wrap_as_needed(formatted_str.strip())


# Example usage:
sexpression_str = "(((FIRST ((((e1 is (Sending PTAG))) ∧ ((Federate of e1 is upstream of federate of e2 via a zero-delay connection)) ∧ ((Tag e1) = (Tag e2))) ∨ ((((e1 is (Receiving NET))) ∨ ((e1 is (Sending STOP_GRN)))) ∧ ((Federate(e1) = Federate(e2)) ∨ ((Federate of e1 is directly upstream of federate of e2))) ∧ ((Tag e1) = (Tag e2)))))) ∧ (((e2 is (Sending PTAG))) ∧ ((Tag e2) ≠ 0))) ⇒ (e1 ≺ e2)"
formatted_sexpression = format_sexpression(sexpression_str)
print(formatted_sexpression)
