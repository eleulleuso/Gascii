description: Sequential Thinking execution protocol (GIGA-CHAD mode)
applyTo: ‘**’
---

# 🔹 Sequential Thinking Rules

## Trigger
- Must execute:
  - When `edit_file` occurs
  - When `reapply` occurs
  - When **architectural/structural decision-making** is required (e.g., stack changes, design choices, security reviews)

---

## Critical Gate A — 3-Round Sequential Thinking

### Round 1 — Problem Analysis
- Define the request's essence (What is the user really asking?)
- Derive requirements & constraints (technology stack, environment, purpose)
- Detect hidden edge cases, bottlenecks, and potential risks
- **Prioritize** (must-have vs nice-to-have)

### Round 2 — Solution Validation
- Present candidate solutions and **compare trade-offs**
- Verify alignment with latest best practices and stack architecture
- Evaluate impact on performance, scalability, security, maintainability
- Reference existing failure cases (memory bank) → Prevent repeating the same mistakes

### Round 3 — Knowledge Assessment
- Verify if current knowledge is sufficient
- Check latest APIs, versions, and deprecation status
- Classify uncertainty level:
  - ✅ High-confidence (Proceed immediately)
  - ⚠️ Medium-confidence (Requires additional verification)
  - ❌ Low-confidence (Requires research)
- Determine whether to call Exa/Context7/web search

---

## Enforcement
- **Final decision and execution only after completing all 3 rounds**
- **Reasoning Log** must be attached to deliverables
- Do not expose raw chain-of-thought; display only summarized logs

### Reasoning Log Template
```md
[Reasoning Log]
R1 Summary: Problem definition, requirements, edge cases
R2 Summary: Review of solution candidates, trade-offs, architectural suitability
R3 Summary: Knowledge sufficiency, recency, need for search (yes/no + reason)
Decision: Final selection X — The reason is …
Confidence: High / Medium / Low


Translated with DeepL.com (free version)