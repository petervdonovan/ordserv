use trace_ord::axioms::axioms;

pub fn main() {
    for axiom in axioms() {
        println!("{}", axiom);
    }
}
