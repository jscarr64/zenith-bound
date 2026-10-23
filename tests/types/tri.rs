//! CONTRACT 15.2, O15-W, D14; oracle: the clauses (three variants; comparison logic is a later unit)
//! COVERS: Tri::True, Tri::False, Tri::Unknown

use zenith_bound::tri::Tri;

fn name(t: &Tri) -> &'static str {
    match t {
        Tri::True => "True",
        Tri::False => "False",
        Tri::Unknown => "Unknown",
    }
}

#[test]
fn tri_variants_are_distinct_and_named() -> Result<(), String> {
    let all = [Tri::True, Tri::False, Tri::Unknown];
    for (i, a) in all.iter().enumerate() {
        if format!("{:?}", a) != name(a) {
            return Err(format!("Debug of {} was {:?}", name(a), a));
        }
        for (j, b) in all.iter().enumerate() {
            if (a == b) != (i == j) {
                return Err(format!("{:?} == {:?} gave {}", a, b, a == b));
            }
        }
    }
    Ok(())
}
