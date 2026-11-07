export type SkillSummary = {
  id: string
  name: string
  description: string
  tags: string[]
  updatedAt: string
}

export type SkillFile = { path: string; content: string }

export const skills: SkillSummary[] = [
  {
    id: 'engineering',
    name: '工程实践',
    description: '让 AI 沿着清晰的边界写代码，把需求落到可维护的实现。',
    tags: ['开发', '代码审查'],
    updatedAt: '2026-06-08',
  },
  {
    id: 'writing',
    name: '技术文档',
    description: '从读者的问题出发，写出简洁、准确、可执行的工程文档。',
    tags: ['写作', '文档'],
    updatedAt: '2026-06-06',
  },
  {
    id: 'interface',
    name: '界面设计',
    description: '为真实任务设计克制的界面，照顾信息层次与交互细节。',
    tags: ['设计', '开发'],
    updatedAt: '2026-06-05',
  },
  {
    id: 'research',
    name: '资料研究',
    description: '拆解研究问题，核对来源，把零散资料整理成有依据的判断。',
    tags: ['研究', '写作'],
    updatedAt: '2026-06-03',
  },
  {
    id: 'debugging',
    name: '问题定位',
    description: '从现象走到根因，用最小实验缩短排查路径。',
    tags: ['开发', '调试'],
    updatedAt: '2026-05-30',
  },
  {
    id: 'planning',
    name: '需求澄清',
    description: '找出真正影响决策的信息缺口，让计划成为下一步行动。',
    tags: ['协作', '研究'],
    updatedAt: '2026-05-28',
  },
]

// 受限内容只在通过权限检查后交给阅读器；全部内容均为本地演示素材。
export function getMockFiles(skill: SkillSummary): SkillFile[] {
  return [
    {
      path: 'README.md',
      content: `# ${skill.name}\n\n${skill.description}\n\n## 使用方式\n\n1. 下载并解压本目录。\n2. 将目录放入项目的 skills 路径。\n3. 在需要时让助手读取 \`SKILL.md\`。\n\n## 目录结构\n\n- \`SKILL.md\`：工作方式与执行边界\n- \`references/checklist.md\`：交付前检查清单\n- \`examples/task.txt\`：一份可直接尝试的任务\n\n## 工作约定\n\n先理解上下文，再选择最小可行方案。遇到会改变结果的缺口，先澄清再行动。\n\n> 这是 Skills Hub 的本地 mock 示例内容，不是正式发布的 skill。\n\n## 版本\n\n只提供最新版本。账号绑定兑换码后，后续更新也可查看与下载。\n`,
    },
    {
      path: 'SKILL.md',
      content: `---\nname: ${skill.id}\ndescription: ${skill.description}\n---\n\n# ${skill.name}\n\n## 工作方式\n\n- 先读当前任务需要的上下文。\n- 说明判断，避免把所有选项交还给用户。\n- 完成最小、可验证的结果，不扩展无关范围。\n\n## 交付\n\n说明完成了什么、如何验证，以及还需要人类确认的事项。\n`,
    },
    {
      path: 'references/checklist.md',
      content:
        '# 交付检查\n\n- [ ] 结果符合用户的实际目标\n- [ ] 没有扩大任务范围\n- [ ] 关键路径已验证\n- [ ] 待确认的问题已经明确列出\n',
    },
    {
      path: 'examples/task.txt',
      content: `示例任务：${skill.name}\n\n请先阅读项目约定，再处理当前任务。\n给出最小可行的结果，并说明验证方式。\n不要修改与目标无关的文件。\n`,
    },
  ]
}
