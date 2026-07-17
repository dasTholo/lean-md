//! Declarative argument schema for lmd directives.
//!
//! One declaration per directive, read by BOTH `check` and the bridge. Two copies of
//! "what is a valid argument" is the drift that produced the bug this fixes: `check`
//! only parsed, so it called `@dispatch brief=x` ok while the bridge never read `brief`
//! at all — the argument fell on the floor without a word.

use crate::args::DirectiveArgs;

/// The argument contract of a single directive.
pub struct ArgSpec {
    /// Alternative argument groups; exactly one must be fully present. The group's
    /// LAST member is its distinguishing key (the one that selects the branch) and
    /// is the name used in the exclusivity message.
    pub required_one_of: &'static [&'static [&'static str]],
    /// Arguments that may appear but are never required.
    pub optional: &'static [&'static str],
    /// Closed value sets, checked per argument name.
    pub enums: &'static [(&'static str, &'static [&'static str])],
}

static DISPATCH: ArgSpec = ArgSpec {
    required_one_of: &[&["phase"], &["skill", "companion"]],
    optional: &["role", "to_agent"],
    enums: &[("role", &["dev", "review", "test"])],
};

pub fn spec(directive: &str) -> Option<&'static ArgSpec> {
    match directive {
        "dispatch" => Some(&DISPATCH),
        _ => None,
    }
}

/// Validate a directive's arguments against its schema. Directives without a schema
/// always pass — the schema is opt-in, not a whitelist of known directives.
///
/// Err(msg) names the offending argument AND the known ones — a user with a typo
/// must not have to guess.
pub fn validate(directive: &str, args: &DirectiveArgs) -> Result<(), String> {
    let Some(spec) = spec(directive) else {
        return Ok(());
    };

    // (1) every NAMED argument must be declared. Positional args are not name-checked
    // — `@dispatch task-1` fills the `phase` group below.
    let known: Vec<&str> = spec
        .required_one_of
        .iter()
        .flat_map(|g| g.iter().copied())
        .chain(spec.optional.iter().copied())
        .collect();
    for (k, _) in args.named_pairs() {
        if !known.contains(&k.as_str()) {
            return Err(format!(
                "@{directive}: unknown argument '{k}' — known: {}",
                known.join(", ")
            ));
        }
    }

    // (2) exactly one group must be fully satisfied. A single-key group also counts as
    // satisfied by positional(0) — `@dispatch task-1` is the documented short form and
    // must not break silently, which is the very failure mode this schema closes.
    let positional = args.positional(0).is_some();
    let satisfied =
        |g: &&[&str]| (g.len() == 1 && positional) || g.iter().all(|k| args.get(k).is_some());
    let label = |g: &&[&str]| format!("{}=", g.last().copied().unwrap_or_default());
    let full = spec.required_one_of.iter().filter(|g| satisfied(g)).count();
    let labels: Vec<String> = spec.required_one_of.iter().map(label).collect();
    if full > 1 {
        return Err(format!(
            "@{directive}: use exactly one of {}",
            labels.join(" or ")
        ));
    }
    if full == 0 {
        // A partially given group is a better error than a generic one: the user
        // clearly meant that branch and only missed a key.
        for g in spec.required_one_of {
            if g.iter().any(|k| args.get(k).is_some()) {
                let missing: Vec<&str> = g
                    .iter()
                    .copied()
                    .filter(|k| args.get(k).is_none())
                    .collect();
                return Err(format!(
                    "@{directive}: missing argument '{}'",
                    missing.join("', '")
                ));
            }
        }
        return Err(format!(
            "@{directive}: missing a required argument — use {}",
            labels.join(" or ")
        ));
    }

    // (3) closed value sets.
    for (key, allowed) in spec.enums {
        if let Some(v) = args.get(key)
            && !allowed.contains(&v)
        {
            return Err(format!(
                "@{directive}: unknown {key} '{v}'. Use: {}",
                allowed.join("|")
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(raw: &str) -> Result<(), String> {
        validate("dispatch", &DirectiveArgs::parse(raw))
    }

    #[test]
    fn a_positional_phase_still_satisfies_the_phase_group() {
        assert!(v("task-1").is_ok());
        assert!(v("task-1 role=review").is_ok());
    }

    #[test]
    fn unknown_named_argument_names_itself_and_the_known_ones() {
        let e = v("brief=x phase=y").unwrap_err();
        assert!(e.contains("unknown argument 'brief'"), "{e}");
        assert!(e.contains("to_agent"), "{e}");
    }

    #[test]
    fn companion_without_skill_reports_the_missing_key() {
        let e = v("companion=c").unwrap_err();
        assert!(e.contains("skill"), "{e}");
        assert!(e.contains("missing"), "{e}");
    }

    #[test]
    fn a_directive_without_a_schema_passes() {
        assert!(validate("read", &DirectiveArgs::parse("anything=1")).is_ok());
    }
}
