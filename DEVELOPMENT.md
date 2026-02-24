# 开发手册

## 插桩规则

### 语句覆盖率（Statement Coverage）

#### 需要插桩的语句类型

以下语句类型需要在执行前插入计数器：

- 表达式语句（ExprStmt）
- 返回语句（ReturnStmt）
- Break 语句（BreakStmt）
- Continue 语句（ContinueStmt）
- Throw 语句（ThrowStmt）
- If 语句（IfStmt）
- Switch 语句（SwitchStmt）
- Try 语句（TryStmt）
- While/DoWhile/For/ForIn/ForOf 循环语句
- Labeled 语句（LabeledStmt）
- With 语句（WithStmt）
- Debugger 语句（DebuggerStmt）

#### 不需要插桩的语句类型

以下语句类型**不应该**被计入语句覆盖率：

1. **函数声明（FunctionDeclaration）**
   ```javascript
   function add(a, b) {  // ← 不插桩
       return a + b;      // ← 需要插桩
   }
   ```
   - 原因：函数声明会被提升（hoisting），在模块加载时就已经定义
   - 函数声明本身不是"可执行语句"，只有函数体内的语句才需要统计
   - 符合 Istanbul 标准行为

2. **Directive 语句**
   ```javascript
   "use strict";  // ← 不插桩
   ```
   - 使用 `stmt.directive_continue()` 判断

3. **插桩生成的语句**
   ```javascript
   cov_xxx().s[0]++;  // ← 不插桩（使用 DUMMY_SP）
   ```
   - 所有插桩生成的语句都使用 `DUMMY_SP`
   - 通过 `span == DUMMY_SP` 判断并跳过

#### 实现示例

```rust
// 在 visit_mut_module_items 中
if let ModuleItem::Stmt(ref stmt) = &item {
    let span = stmt.span();
    
    // 跳过条件
    let should_skip = span == DUMMY_SP 
        || stmt.directive_continue()
        || matches!(stmt, Stmt::Decl(Decl::Fn(_)));  // 跳过函数声明
    
    if !should_skip {
        let counter = self.mark_prepend_stmt_counter(&span);
        new_items.push(ModuleItem::Stmt(counter));
    }
}
```

### 避免重复插桩

#### 问题描述

插桩过程中会生成新的语句（如 `cov_xxx().s[0]++`），如果不加控制，这些语句可能会被再次插桩，导致：
- 覆盖率数据污染
- 无限递归插桩
- 生成的代码体积膨胀

#### 解决方案

使用 `DUMMY_SP`（dummy span）标记所有插桩生成的语句：

```rust
fn mark_prepend_stmt_counter(&self, span: &Span) -> Stmt {
    let range = self.get_range(span);
    let id = self.cov.borrow_mut().new_statement(&range);
    
    Stmt::Expr(ExprStmt {
        span: DUMMY_SP,  // ← 使用 DUMMY_SP
        expr: Box::new(create_increase_counter_expr(id, &self.cov_fn_ident)),
    })
}
```

在遍历语句时检查并跳过：

```rust
if span != DUMMY_SP {
    // 只对原始代码插桩
    let counter = self.mark_prepend_stmt_counter(&span);
    new_stmts.push(counter);
}
```

### 函数覆盖率 vs 语句覆盖率

#### 区别

| 类型 | 统计内容 | 数据结构 | 示例 |
|------|---------|---------|------|
| 函数覆盖率 | 函数是否被调用 | `fnMap`, `f` | `function add() {}` |
| 语句覆盖率 | 语句是否被执行 | `statementMap`, `s` | `return a + b;` |

#### 函数声明的处理

```javascript
// 源代码
function add(a, b) {
    return a + b;
}
console.log(add(1, 2));

// 插桩后
// 注意：函数声明本身没有 counter
function add(a, b) {
    cov_xxx().s[0]++;  // ← 只有函数体内的语句有 counter
    return a + b;
}
cov_xxx().s[1]++;      // ← 顶层语句有 counter
console.log(add(1, 2));
```

生成的覆盖率数据：

```javascript
{
    statementMap: {
        "0": { start: { line: 2, column: 4 }, end: { line: 2, column: 17 } },  // return 语句
        "1": { start: { line: 5, column: 0 }, end: { line: 5, column: 22 } }   // console.log
    },
    s: { "0": 0, "1": 0 }
    // 注意：没有函数声明的语句记录
}
```

### 调试技巧

#### 打印语句信息

在关键位置添加打印语句，帮助理解插桩过程：

```rust
fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
    println!("=== visit_mut_stmts: 处理 {} 个块语句 ===", stmts.len());
    
    for (idx, mut stmt) in stmts.drain(..).enumerate() {
        let span = stmt.span();
        
        if span != DUMMY_SP {
            println!("  [{}] 块语句: {:?}", idx, stmt);
            let range = self.get_range(&span);
            println!("    -> 注入 counter, range: {:?}", range);
        } else {
            println!("  [{}] 跳过 DUMMY_SP 语句（插桩生成）", idx);
        }
    }
}
```

#### 验证输出

运行插桩后，检查生成的代码：

```bash
cd playground/swc
pnpm swc ./src -d dist
cat dist/src/file.js
```

确认：
1. 函数声明前没有 counter
2. 函数体内的语句有 counter
3. 顶层语句有 counter
4. 没有重复的 counter

## 参考资料

- [Istanbul 官方文档](https://istanbul.js.org/)
- [babel-plugin-istanbul 源码](https://github.com/istanbuljs/babel-plugin-istanbul)
- [SWC AST 文档](https://swc.rs/docs/plugin/ecmascript/ast)

## 代码结构

### 模块组织

```
packages/swc-coverage-instrument/src/
├── lib.rs                      # 公共 API
├── source_coverage.rs          # 覆盖率数据结构
├── coverage_template.rs        # 覆盖率初始化模板（新）
└── visitors/
    ├── mod.rs
    └── coverage_visitor.rs     # 插桩 Visitor
```

### coverage_template.rs

负责生成覆盖率初始化代码，包括：

1. **create_coverage_data_object** - 创建覆盖率数据对象
   ```javascript
   {
     path: "src/file.js",
     statementMap: { "0": { start, end }, ... },
     fnMap: {},
     branchMap: {},
     s: { "0": 0, "1": 0 },
     f: {},
     b: {}
   }
   ```

2. **create_global_stmt_template** - 创建全局对象获取语句
   ```javascript
   var global = new Function("return this")();
   ```

3. **create_coverage_fn_decl** - 创建覆盖率函数声明
   ```javascript
   global.__coverage__ = global.__coverage__ || {};
   global.__coverage__["path"] = { /* data */ };
   cov_xxx = function() { return global.__coverage__["path"]; };
   ```

### coverage_visitor.rs

负责遍历 AST 并插入计数器：

1. **visit_mut_program** - 在程序顶部插入初始化代码
2. **visit_mut_module_items** - 处理模块级语句
3. **visit_mut_script** - 处理脚本级语句
4. **visit_mut_stmts** - 处理块级语句

### 重构优势

相比旧的实现，新的结构：

1. **模块化** - 模板生成逻辑独立到 coverage_template.rs
2. **可维护** - 代码结构清晰，职责分明
3. **可扩展** - 未来添加函数/分支覆盖率更容易
4. **可测试** - 模板生成可以独立测试

## 参考资料

- [Istanbul 官方文档](https://istanbul.js.org/)
- [babel-plugin-istanbul 源码](https://github.com/istanbuljs/babel-plugin-istanbul)
- [SWC AST 文档](https://swc.rs/docs/plugin/ecmascript/ast)
