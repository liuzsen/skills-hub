# 代码风格

先展示整体意图，再按需展开细节。抽象、类型和拆分带来的理解、维护或安全收益，应抵得过新增的复杂度。

## 语义层次与控制流

函数主体展示当前层次的意图，复杂细节下沉到有明确语义的函数。即使只调用一次，只要名字提供了真正的理解捷径，也值得提取。

让 match / switch 等分派位置展示分支全貌，复杂分支提取为函数，简单返回、转换和短小处理留在原处。

```rust
match command {
    Command::Renew(input) => renew(input).await,
    Command::GetStatus => Ok(current_status),
}
```

## 模块与职责

可独立理解和修改的职责分开，需一起阅读的代码相邻。

避免承载多种独立职责的万能 Service / Manager / Context；仅把同一对象的方法搬到多个文件不算职责拆分。业务操作显式依赖所需能力，不传入万能上下文隐藏依赖；组装期集中持有资源的上下文不受此限。

类型归维护其语义与规则的模块，不按 struct、enum、trait 等代码形状集中归档为 `types.rs`/`types.ts`。模块入口可以定义本模块共用的核心类型、trait 和少量逻辑；有独立语义的共同能力自成模块。

例如，订单模块的共用契约留在 `mod.rs`，创建与取消流程分别阅读；权限规则有独立语义，单独成模块：

```text
orders/
  mod.rs          # OrderId、OrderRepository trait、模块声明与导出
  create.rs       # 创建订单
  cancel.rs       # 取消订单
  permissions.rs  # 多个操作共用的订单权限规则
```

通用描述与独立演化的具体实例分开组织，同一实例的数据和专用行为相邻。例如，插件描述类型与注册入口留在 `plugins/mod.rs`，各插件的配置与专用逻辑归各自模块。

## 表达式与命名

参数表达式过长、嵌套过深，或中间结果有独立含义时，先绑定有意义的局部变量。即使链式调用更短，变量能明显帮助理解时也优先变量。

例如，与其在参数里嵌套解析、转换和格式化：

```rust
send_email(user.email(), &render_renewal_notice(&plan, calculate_renewal_price(&plan, today)?)).await?;
```

不如让中间结果说明业务含义：

```rust
let renewal_price = calculate_renewal_price(&plan, today)?;
let notice = render_renewal_notice(&plan, renewal_price);
send_email(user.email(), &notice).await?;
```

## 类型与边界

用明确类型表达已知数据结构。有实际约束使用 newtype，普通文本可以使用 `String`。用 enum / discriminated union 表达互斥状态，各状态携带所需数据，不用布尔值与可选字段拼凑合法状态。

例如，不使用可能矛盾的 `{ loading: boolean; data?: User; error?: string }`，而是：

```typescript
type UserState =
  | { kind: "loading" }
  | { kind: "ready"; user: User }
  | { kind: "failed"; error: LoadUserError };
```

在防腐层边界校验并解析为静态类型，内部信任已建立的约束。动态 JSON（如 `serde_json::Value`）只用于结构确实动态、未知或需要透传的部分。

## 复用与现成能力

独立演化的逻辑可以重复，共同维护的规则应共享，不以减少重复行数抵偿显著增加的理解成本。

例如，创建用户与创建团队都要“校验 → 保存 → 返回”，仍可各自实现；不为消除这段相似流程引入 `create_with(validate, persist, map_output)`。两者若都遵循同一条租户配额规则，则下沉并共享该规则。

UI 交互、命令行解析、协议解析等标准机制，先考虑成熟库，自行实现应有明确的需求或成本理由。
