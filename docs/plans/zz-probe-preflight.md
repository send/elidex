# probe

## §3 Spec coverage map

| Spec section | Step | Branch | Touch (compile/dispatch site) | Full enum? | User-input flow |
|---|---|---|---|---|---|
| `scripts/trip-wires.sh` §1 (the driver) | discover | (i) glob | `WIRE_GLOB` | ✓ | no |

## §8.3 Preflight

```
  total entries  (M):   1  (data rows, breadth basis)
  malformed rows:       0  (no §<number> reference)
  unmapped-label rows:  1  (has §<number>, label not in SPEC_LABEL_REVERSE)
  unique specs (K):     1 (<`scripts/trip-wires.sh`>)
  grep-pass:            0 hard, 0 soft (spec: m4-12-pr-elidex-plan-review-grep-pass-spec.md)
EXIT=0
```
