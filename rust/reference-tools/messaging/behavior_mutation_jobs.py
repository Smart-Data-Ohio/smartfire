"""Plan served probes without sharing writing mutants' fixture databases."""


def probe_jobs(batches, selected, *, negative=False, mutant=None,
               mutation_variants=None, diagnostic_variants=None):
    if diagnostic_variants is not None:
        variants = mutation_variants if negative else diagnostic_variants
        return [([case], variant) for case in selected for variant in variants[case]]
    if not negative:
        return [(batch, mutant or "default") for batch in batches if batch]

    # Read-only defaults may share a server, but a named-only case must never
    # enter that batch under an unregistered default (or selected variant).
    variant = mutant or "default"
    jobs = [(eligible, variant) for batch in batches
            if (eligible := [case for case in batch
                             if variant in mutation_variants[case]])]
    if not mutant:
        # Each further mutant gets an independent fixture: a failed assertion
        # can still leave real writes in its database.
        jobs += [([case], name) for case in selected
                 for name in mutation_variants[case] if name != "default"]
    return jobs
