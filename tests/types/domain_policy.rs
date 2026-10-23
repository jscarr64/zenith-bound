//! CONTRACT 10.1, 15.1, O6; oracle: the clauses (two variants, Strict is Default)
//! COVERS: DomainPolicy::Strict, DomainPolicy::Clip, Default for DomainPolicy

use zenith_bound::policy::DomainPolicy;

fn name(p: &DomainPolicy) -> &'static str {
    match p {
        DomainPolicy::Strict => "Strict",
        DomainPolicy::Clip => "Clip",
    }
}

#[test]
fn default_is_strict() -> Result<(), String> {
    match DomainPolicy::default() {
        DomainPolicy::Strict => Ok(()),
        DomainPolicy::Clip => Err("default was Clip".into()),
    }
}

#[test]
fn variants_are_distinct_and_named() -> Result<(), String> {
    let all = [DomainPolicy::Strict, DomainPolicy::Clip];
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
    if DomainPolicy::default() != DomainPolicy::Strict {
        return Err("default != Strict".into());
    }
    Ok(())
}
