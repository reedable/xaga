# Strengthen embedding-model trust anchoring

## Summary

The pinned model manifest was derived through one machine and network, and the claimed derivation from the upstream sentence-transformers model has not been reproduced. Decide whether either trust-chain gap warrants additional anchoring work.

## Requirements

- Treat independent-network verification and reproducible upstream export as separate hardening options.
- Record the evidence each option adds and the trust it cannot remove.
- Pin any reproduction toolchain and inputs sufficiently to repeat the comparison.
- Do not change the manifest without reconciling the result with the durable trust-chain documentation.

## Remaining work

- Estimate and compare independent-network re-anchoring and reproducible-export work.
- If independent verification is selected, obtain and compare the pinned files through a genuinely independent machine and network path.
- If reproduction is selected, define and run the pinned export procedure and compare its outputs or behavior with the shipped artifact.
- Update the anchoring procedure and trust limits with the selected evidence.

## References

- `crates/xaga-core/src/embeddings/model.rs`
- `docs/architecture/security/embedding_model_trust.md`
