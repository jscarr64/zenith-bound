//! CONTRACT 6.4, 15.3, O11-W; oracle: the clauses (the variant set, derives, Display and Error)
//! COVERS: BoundError::InvalidArgument, BoundError::Domain, BoundError::Resource, Display for BoundError, Error for BoundError

use zenith_bound::error::{BoundError, BoundResult};

fn all() -> [BoundError; 4] {
    [
        BoundError::Representation,
        BoundError::InvalidArgument,
        BoundError::Domain,
        BoundError::Resource,
    ]
}

fn name(e: &BoundError) -> &'static str {
    match e {
        BoundError::Representation => "Representation",
        BoundError::InvalidArgument => "InvalidArgument",
        BoundError::Domain => "Domain",
        BoundError::Resource => "Resource",
    }
}

#[test]
fn every_variant_equals_only_itself() -> Result<(), String> {
    for (i, a) in all().iter().enumerate() {
        for (j, b) in all().iter().enumerate() {
            if (a == b) != (i == j) {
                return Err(format!("{:?} == {:?} gave {}", a, b, a == b));
            }
        }
    }
    Ok(())
}

#[test]
fn debug_names_the_variant() -> Result<(), String> {
    for e in all() {
        if format!("{:?}", e) != name(&e) {
            return Err(format!("Debug of {} was {:?}", name(&e), e));
        }
    }
    Ok(())
}

#[test]
fn display_is_nonempty_and_distinct() -> Result<(), String> {
    let shown: Vec<(String, &str)> = all().iter().map(|e| (format!("{}", e), name(e))).collect();
    for (i, (a, n)) in shown.iter().enumerate() {
        if a.is_empty() {
            return Err(format!("empty Display for {}", n));
        }
        for (j, (b, _)) in shown.iter().enumerate() {
            if j > i && a == b {
                return Err(format!("duplicate Display {:?}", a));
            }
        }
    }
    Ok(())
}

#[test]
fn bound_error_is_a_core_error() -> Result<(), String> {
    let boxed: Box<dyn core::error::Error> = Box::new(BoundError::Domain);
    if boxed.source().is_some() {
        return Err("BoundError has no source".into());
    }
    if boxed.to_string() != format!("{}", BoundError::Domain) {
        return Err("Error::to_string differs from Display".into());
    }
    Ok(())
}

#[test]
fn bound_result_is_result_of_bound_error() -> Result<(), String> {
    let ok: BoundResult<u32> = Ok(7);
    let err: BoundResult<u32> = Err(BoundError::Resource);
    if Err::<u32, _>(BoundError::InvalidArgument) == Err(BoundError::Domain) {
        return Err("InvalidArgument equals Domain".into());
    }
    let plain: Result<u32, BoundError> = ok;
    match (plain, err) {
        (Ok(7), Err(BoundError::Resource)) => Ok(()),
        (a, b) => Err(format!("got {:?} {:?}", a, b)),
    }
}
