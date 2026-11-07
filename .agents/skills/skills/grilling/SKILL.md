---
name: grilling
description: Clarify plans and decisions, and resolve task-critical information gaps that require human input. Use when the user requests a stress test or progress depends on a human decision or information unavailable to the agent.
---

Interview the user relentlessly until you reach a shared understanding. Map this as a **design tree**: every decision branches into the decisions that hang off it. Identify missing information needed to settle those decisions.

Work the tree in **rounds**. The **frontier** is every decision whose prerequisites are already settled — the questions you can ask _now_ without guessing at answers you haven't heard yet. Ask the whole frontier in one round: number each question and give your recommended answer, except when asking for missing information. Then wait for the user's answers before the next round.

At the start of the interview, at the first round, tell the user: **If you agree with every recommended answer in a round, reply `ok`. If you disagree with any recommendation, answer those questions by number; unanswered decision questions default to the recommended answer.** Apply this convention throughout the interview.

Number new questions consecutively across the entire conversation.

Use 🔀 for decisions and ❓ for missing information. Format decision questions like so; information questions use the same format without a recommended answer:

```
🔀 **Q1** - **<question title>**: <question body, might be multiple paragraphs, including multiple choices>

➡️ <your recommended answer>
```

Each round the user answers reshapes the tree — settled decisions push the frontier outward and unblock questions that depended on them. Recompute the frontier and ask the next round. A question whose answer depends on another question still open in this round belongs to a _later_ round, not this one.

Finding accessible _facts_ is your job. When a frontier question needs a fact from the environment (filesystem, tools, etc.), dispatch a sub-agent to find it — don't ask the user for anything you could look up yourself. When necessary information cannot be reliably obtained from accessible sources, ask the user. Don't block on it: a running exploration or missing information is an unsettled prerequisite, so only the questions downstream of it wait — ask the rest of the frontier now. The _decisions_ are the user's — put each to them and wait.

The interview is done when the frontier is empty: every branch of the design tree visited, nothing left silently assumed.

## During the interview

### Challenge against the project language

Terms live in `docs/zsen/`: global language in `index.md`, local language in the routed Domain document — the same word may legitimately mean different things in different Domains, so check the local language before flagging a conflict. When the user's usage clashes with a defined term, call it out immediately and name where it is defined. "The constitution defines 'cancellation' as X in settlement, but you seem to mean Y — which is it?"

### Challenge against commitments

When the user's plan conflicts with an accepted commitment, boundary, or constraint in the constitution, surface it as a governance choice instead of quietly routing around it: "The constitution commits to X; this plan breaks it — do you want to change the commitment, or the plan?" Changing an accepted commitment needs the same authority as creating one; deployed code alone never settles it. Conversely, when stable code behavior has no commitment behind it, ask whether it should become one.

### Sharpen fuzzy language

When the user uses vague or overloaded terms, propose a precise canonical term. "You're saying 'account' — do you mean the Customer or the User? Those are different things."

### Discuss concrete scenarios

When domain relationships are being discussed, stress-test them with specific scenarios. Invent scenarios that probe edge cases and force the user to be precise about the boundaries between concepts.

### Cross-reference with code

When the user states how something works, check whether the code agrees. If you find a contradiction, surface it: "Your code cancels entire Orders, but you just said partial cancellation is possible — which is right?"
