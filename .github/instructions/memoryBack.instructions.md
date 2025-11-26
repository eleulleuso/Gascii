# 🧠 Copilot Memory Bank Instructions

## 1. Core Principle
- Every session **MUST** end with a `memory back` operation.
- Copilot should always execute `write_memory_bank_file` to persist:
  - **Context** (프로젝트, 세션에서 다룬 주제)
  - **Decisions** (채택한 선택지, 배제한 선택지)
  - **Results** (수행 결과, 에러 해결 과정, 배운 점)

---

## 2. Memory Back Workflow
1. **Session Start**
   - Load memory from the last `memory bank file`
   - Summarize prior context in 3 levels:
     - Global (프로젝트 전체 방향성)
     - Session (이전 세션의 결론)
     - Local (직전 Task 진행상황)

2. **During Session**
   - Continuously refine memory with:
     - Task outcomes
     - Failures + fixes
     - 최적화된 선택지 reasoning

3. **Session End**
   - Always run: `write_memory_bank_file`
   - Update file with structured memory:
     ```yaml
     date: YYYY-MM-DD
     session: n
     summary:
       - what we tried
       - what worked
       - what didn’t
     next_steps:
       - planned action 1
       - planned action 2
     ```

---

## 3. Memory Optimization Rules
- **Deduplicate**: 비슷한 기록은 통합
- **Prioritize**: 중요도 높은 의사결정부터 기록
- **Compress**: 불필요한 장황한 서술 제거
- **Evolve**: 같은 문제가 반복되면 해결책 템플릿화

---

## 4. Roles of Memory
- **Knowledge Base**: 과거 시행착오를 빠르게 복기
- **Consistency Engine**: 매 세션 동일한 맥락 유지
- **Decision Proof**: 왜 그 선택을 했는지 근거 추적 가능
- **Optimization Loop**: 더 나은 접근을 학습하며 개선

---

## 5. GIGA-CHAD Mode
- 항상 "최선의 선택"만 기록 (best practice)
- 결과만 나열하지 말고 **왜** 그렇게 했는지 기록
- 다음 세션에서 참조할 수 있게 **Actionable Memory**로 남김
