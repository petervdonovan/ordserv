use trace_ord::{axioms::axioms, lflib::syntax_explanation_for_llm};

pub fn main() {
    println!("{}", syntax_explanation_for_llm());
    //     println!(
    //         "Please explain what the following propositions mean. Start by stating the kinds of messages that are being sent or received in e1 and e2, respectively.
    // "
    //     );
    println!("Each of the following propositions is of the form $A \\implies e1 \\prec e2$. For each of the propositions, please do the following things:
1. State what kinds of messages are being sent or received in e1 and e2, respectively.
2. Explain what the proposition means.
");
    for axiom in axioms() {
        println!("{}", axiom);
    }
}
