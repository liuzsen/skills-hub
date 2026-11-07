---
name: handoff
description: 生成跨 agent 会话的交接文档，由用户手动调用并传入交接用途。
disable-model-invocation: true
---

# 会话交接

接收方是对当前对话一无所知的 agent，用户传入的参数定义其工作，需要重点关注。

按接收方的需要蒸馏当前上下文，写入 `.local/.zsen/handoff/<slug>.md`。

完成时报告文档路径。
